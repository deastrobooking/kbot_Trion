//! Bounded role-based command authority (TR-P4-042, TR-P4-050).
//!
//! Authentication is a trust boundary: an external identity provider verifies
//! credentials and creates an [`AuthContext`]. This module enforces roles,
//! monotonic anti-replay sequences, and expiring two-step approvals; it does
//! not implement passwords, certificates, or token signature verification.

use crate::telemetry::events::{EventKind, EventLog};
use crate::time::{duration_ms, RuntimeClock};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Observer,
    Operator,
    Supervisor,
}

impl Role {
    fn permits(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }

    fn rank(self) -> u8 {
        match self {
            Self::Observer => 0,
            Self::Operator => 1,
            Self::Supervisor => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthContext {
    subject: String,
    role: Role,
    authenticated: bool,
}

impl AuthContext {
    /// Construct a trusted context only after an external authentication
    /// adapter has verified the subject's credentials and assigned role.
    pub fn authenticated(subject: impl Into<String>, role: Role) -> eyre::Result<Self> {
        let subject = subject.into();
        if subject.trim().is_empty() {
            return Err(eyre::eyre!("authenticated subject must not be empty"));
        }
        Ok(Self {
            subject,
            role,
            authenticated: true,
        })
    }

    pub fn unauthenticated(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            role: Role::Observer,
            authenticated: false,
        }
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandClass {
    /// E-stop and entry to safe-mode. Any authenticated role may invoke it.
    Immediate,
    /// Ordinary task or teleoperation command. Requires operator role.
    Queued,
    /// Hazardous/contact command. Requires operator role and a live approval.
    CommitWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub id: String,
    pub sequence: u64,
    pub class: CommandClass,
    pub action: String,
    pub approval_token: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommandDecision {
    command_id: String,
    subject: String,
    class: CommandClass,
}

impl CommandDecision {
    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn class(&self) -> CommandClass {
        self.class
    }
}

struct Approval {
    command_id: String,
    approved_by: String,
    expires_at_ms: u64,
}

pub struct CommandAuthority {
    clock: RuntimeClock,
    events: EventLog,
    commit_window_ms: u64,
    approval_capacity: usize,
    principal_capacity: usize,
    approvals: HashMap<String, Approval>,
    last_sequence: HashMap<String, u64>,
}

impl CommandAuthority {
    pub fn new(
        clock: RuntimeClock,
        events: EventLog,
        commit_window: Duration,
        approval_capacity: usize,
        principal_capacity: usize,
    ) -> eyre::Result<Self> {
        if commit_window.is_zero() {
            return Err(eyre::eyre!("commit window must be greater than zero"));
        }
        if approval_capacity == 0 || principal_capacity == 0 {
            return Err(eyre::eyre!(
                "authority capacities must be greater than zero"
            ));
        }
        Ok(Self {
            clock,
            events,
            commit_window_ms: duration_ms(commit_window),
            approval_capacity,
            principal_capacity,
            approvals: HashMap::with_capacity(approval_capacity),
            last_sequence: HashMap::with_capacity(principal_capacity),
        })
    }

    pub fn arm(
        &mut self,
        approver: &AuthContext,
        command_id: &str,
        approval_token: &str,
        hazard: &str,
    ) -> eyre::Result<()> {
        self.arm_at(
            approver,
            command_id,
            approval_token,
            hazard,
            self.clock.now_ms(),
        )
    }

    fn arm_at(
        &mut self,
        approver: &AuthContext,
        command_id: &str,
        approval_token: &str,
        hazard: &str,
        now_ms: u64,
    ) -> eyre::Result<()> {
        self.require_authenticated(approver)?;
        self.require_role(approver, Role::Supervisor)?;
        if command_id.trim().is_empty()
            || approval_token.trim().is_empty()
            || hazard.trim().is_empty()
        {
            return Err(eyre::eyre!(
                "arming requires command id, approval token, and hazard"
            ));
        }
        self.approvals
            .retain(|_, approval| approval.expires_at_ms >= now_ms);
        if self.approvals.contains_key(approval_token) {
            return Err(eyre::eyre!("approval token is already active"));
        }
        if self.approvals.len() >= self.approval_capacity {
            return Err(eyre::eyre!("approval capacity exhausted"));
        }
        if self
            .approvals
            .values()
            .any(|approval| approval.command_id == command_id)
        {
            return Err(eyre::eyre!("command already has an active approval"));
        }

        self.approvals.insert(
            approval_token.to_owned(),
            Approval {
                command_id: command_id.to_owned(),
                approved_by: approver.subject.clone(),
                expires_at_ms: now_ms.saturating_add(self.commit_window_ms),
            },
        );
        self.events.record(
            EventKind::CommandArmed,
            Some(approver.subject()),
            format!("command '{command_id}' armed for hazard '{hazard}'"),
        );
        Ok(())
    }

    pub fn authorize(
        &mut self,
        principal: &AuthContext,
        request: CommandRequest,
    ) -> eyre::Result<CommandDecision> {
        self.authorize_at(principal, request, self.clock.now_ms())
    }

    fn authorize_at(
        &mut self,
        principal: &AuthContext,
        request: CommandRequest,
        now_ms: u64,
    ) -> eyre::Result<CommandDecision> {
        if let Err(error) = self.check_authority(principal, &request, now_ms) {
            self.events.record(
                EventKind::CommandRejected,
                Some(principal.subject()),
                format!("command '{}' rejected: {error}", request.id),
            );
            return Err(error);
        }

        if request.class == CommandClass::CommitWindow {
            let token = request
                .approval_token
                .as_deref()
                .ok_or_else(|| eyre::eyre!("commit-window command requires approval"))?;
            self.approvals.remove(token);
        }
        if request.class != CommandClass::Immediate {
            self.last_sequence
                .insert(principal.subject.clone(), request.sequence);
        }
        self.events.record(
            EventKind::CommandAuthorized,
            Some(principal.subject()),
            format!(
                "command '{}' authorized as {:?}: {}",
                request.id, request.class, request.action
            ),
        );
        Ok(CommandDecision {
            command_id: request.id,
            subject: principal.subject.clone(),
            class: request.class,
        })
    }

    fn check_authority(
        &self,
        principal: &AuthContext,
        request: &CommandRequest,
        now_ms: u64,
    ) -> eyre::Result<()> {
        self.require_authenticated(principal)?;
        if request.id.trim().is_empty() || request.action.trim().is_empty() {
            return Err(eyre::eyre!("command id and action must not be empty"));
        }
        if request.class != CommandClass::Immediate {
            if let Some(last) = self.last_sequence.get(principal.subject()) {
                if request.sequence <= *last {
                    return Err(eyre::eyre!(
                        "replayed or out-of-order sequence {} (last {})",
                        request.sequence,
                        last
                    ));
                }
            } else if self.last_sequence.len() >= self.principal_capacity {
                return Err(eyre::eyre!("principal replay-state capacity exhausted"));
            }
        }

        match request.class {
            CommandClass::Immediate => Ok(()),
            CommandClass::Queued => self.require_role(principal, Role::Operator),
            CommandClass::CommitWindow => {
                self.require_role(principal, Role::Operator)?;
                let token = request
                    .approval_token
                    .as_deref()
                    .ok_or_else(|| eyre::eyre!("commit-window command requires approval"))?;
                let approval = self
                    .approvals
                    .get(token)
                    .ok_or_else(|| eyre::eyre!("approval is missing or already consumed"))?;
                if approval.command_id != request.id {
                    return Err(eyre::eyre!("approval is bound to a different command"));
                }
                if approval.approved_by.trim().is_empty() {
                    return Err(eyre::eyre!("approval has no authenticated approver"));
                }
                if now_ms > approval.expires_at_ms {
                    return Err(eyre::eyre!("approval window expired"));
                }
                Ok(())
            }
        }
    }

    fn require_authenticated(&self, principal: &AuthContext) -> eyre::Result<()> {
        if !principal.authenticated {
            return Err(eyre::eyre!("command subject is not authenticated"));
        }
        Ok(())
    }

    fn require_role(&self, principal: &AuthContext, required: Role) -> eyre::Result<()> {
        if !principal.role.permits(required) {
            return Err(eyre::eyre!(
                "role {:?} does not permit {:?} operation",
                principal.role,
                required
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority(window: Duration) -> eyre::Result<CommandAuthority> {
        let clock = RuntimeClock::new();
        CommandAuthority::new(clock, EventLog::new(clock, 64)?, window, 4, 4)
    }

    fn request(id: &str, sequence: u64, class: CommandClass) -> CommandRequest {
        CommandRequest {
            id: id.to_owned(),
            sequence,
            class,
            action: "test action".to_owned(),
            approval_token: None,
        }
    }

    #[test]
    fn immediate_safety_command_accepts_any_authenticated_role() -> eyre::Result<()> {
        let mut authority = authority(Duration::from_secs(5))?;
        let observer = AuthContext::authenticated("observer", Role::Observer)?;
        authority.authorize(&observer, request("estop-1", 1, CommandClass::Immediate))?;
        authority.authorize(&observer, request("estop-1", 1, CommandClass::Immediate))?;
        let anonymous = AuthContext::unauthenticated("anonymous");
        assert!(authority
            .authorize(&anonymous, request("estop-2", 1, CommandClass::Immediate))
            .is_err());
        Ok(())
    }

    #[test]
    fn queued_commands_require_operator_and_reject_replay() -> eyre::Result<()> {
        let mut authority = authority(Duration::from_secs(5))?;
        let observer = AuthContext::authenticated("observer", Role::Observer)?;
        assert!(authority
            .authorize(&observer, request("queue-1", 1, CommandClass::Queued))
            .is_err());
        let operator = AuthContext::authenticated("operator", Role::Operator)?;
        authority.authorize(&operator, request("queue-2", 7, CommandClass::Queued))?;
        assert!(authority
            .authorize(&operator, request("queue-3", 7, CommandClass::Queued))
            .is_err());
        assert!(authority
            .authorize(&operator, request("queue-4", 6, CommandClass::Queued))
            .is_err());
        Ok(())
    }

    #[test]
    fn commit_window_requires_supervisor_arming_and_is_one_shot() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 64)?;
        let mut authority =
            CommandAuthority::new(clock, events.clone(), Duration::from_secs(5), 4, 4)?;
        let operator = AuthContext::authenticated("operator", Role::Operator)?;
        let supervisor = AuthContext::authenticated("supervisor", Role::Supervisor)?;
        let mut command = request("contact-1", 1, CommandClass::CommitWindow);
        assert!(authority.authorize(&operator, command.clone()).is_err());
        assert!(authority
            .arm(&operator, "contact-1", "approval-1", "contact motion")
            .is_err());
        authority.arm(&supervisor, "contact-1", "approval-1", "contact motion")?;
        let mut mismatched = request("contact-other", 1, CommandClass::CommitWindow);
        mismatched.approval_token = Some("approval-1".to_owned());
        assert!(authority.authorize(&operator, mismatched).is_err());
        command.approval_token = Some("approval-1".to_owned());
        authority.authorize(&operator, command)?;

        let mut replay_token = request("contact-2", 2, CommandClass::CommitWindow);
        replay_token.approval_token = Some("approval-1".to_owned());
        assert!(authority.authorize(&operator, replay_token).is_err());
        let log = events.to_jsonl();
        assert!(log.contains("CommandArmed"));
        assert!(log.contains("CommandAuthorized"));
        assert!(log.contains("CommandRejected"));
        Ok(())
    }

    #[test]
    fn expired_commit_window_is_rejected_deterministically() -> eyre::Result<()> {
        let mut authority = authority(Duration::from_millis(10))?;
        let operator = AuthContext::authenticated("operator", Role::Operator)?;
        let supervisor = AuthContext::authenticated("supervisor", Role::Supervisor)?;
        authority.arm_at(
            &supervisor,
            "contact-1",
            "approval-1",
            "contact motion",
            100,
        )?;
        let mut command = request("contact-1", 1, CommandClass::CommitWindow);
        command.approval_token = Some("approval-1".to_owned());
        assert!(authority.authorize_at(&operator, command, 111).is_err());
        Ok(())
    }

    #[test]
    fn approval_and_principal_state_are_bounded() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 32)?;
        let mut authority = CommandAuthority::new(clock, events, Duration::from_secs(5), 1, 1)?;
        let supervisor = AuthContext::authenticated("supervisor", Role::Supervisor)?;
        authority.arm(&supervisor, "one", "token-one", "hazard")?;
        assert!(authority
            .arm(&supervisor, "two", "token-two", "hazard")
            .is_err());

        let first = AuthContext::authenticated("first", Role::Operator)?;
        authority.authorize(&first, request("queue-1", 1, CommandClass::Queued))?;
        let second = AuthContext::authenticated("second", Role::Operator)?;
        assert!(authority
            .authorize(&second, request("queue-2", 1, CommandClass::Queued))
            .is_err());
        Ok(())
    }
}
