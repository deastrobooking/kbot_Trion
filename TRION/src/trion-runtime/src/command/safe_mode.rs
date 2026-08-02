//! Safe-mode state machine (TR-P4-020, TR-P4-021). Entering safe-mode flips a
//! watch channel that actuator-facing services observe to command their
//! zero-torque/brake posture; exit happens only on an explicit recovery
//! command, never automatically.

use crate::telemetry::events::{EventKind, EventLog};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Nominal,
    Safe,
}

#[derive(Clone)]
pub struct ModeController {
    tx: Arc<watch::Sender<RunMode>>,
    events: EventLog,
}

impl ModeController {
    pub fn new(events: EventLog) -> Self {
        let (tx, _) = watch::channel(RunMode::Nominal);
        Self {
            tx: Arc::new(tx),
            events,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<RunMode> {
        self.tx.subscribe()
    }

    pub fn mode(&self) -> RunMode {
        *self.tx.borrow()
    }

    pub fn is_safe(&self) -> bool {
        self.mode() == RunMode::Safe
    }

    /// Enter safe-mode (TR-P4-020). Idempotent; the reason is recorded as the
    /// evidence trail for the safety case.
    pub fn enter_safe(&self, reason: &str) {
        if self.is_safe() {
            return;
        }
        self.tx.send_replace(RunMode::Safe);
        self.events
            .record(EventKind::SafeModeEntered, None, reason.to_owned());
    }

    /// Exit safe-mode only via an explicit recovery command (TR-P4-021). The
    /// operator identity is recorded for the audit trail; authentication is
    /// enforced by the command layer above (TR-P4-050).
    pub fn recover(&self, operator: &str) {
        if !self.is_safe() {
            return;
        }
        self.tx.send_replace(RunMode::Nominal);
        self.events.record(
            EventKind::SafeModeExited,
            None,
            format!("recovery commanded by {operator}"),
        );
    }
}
