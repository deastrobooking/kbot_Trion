//! W1 panel-inspection runner (T-01).
//!
//! The pinned K-Bot model has fixed inspection cameras rather than a
//! pan/tilt head. This runner commands bounded arm-clearance postures through
//! command authority and the safety shim, then asks a non-safety-critical
//! observer adapter to capture named W1 targets.

use crate::{AutonomyLevel, PlanStep, SkillPoll, SkillRunner};
use serde::{Deserialize, Serialize};
use trion_runtime::{
    ActuatorCommand, ActuatorTransport, AuthContext, CommandAuthority, CommandClass,
    CommandRequest, KosSafetyShim, Role,
};

const SKILL_NAME: &str = "inspect_fixture";
const PANEL_TARGET: &str = "panel";
const HANDLE_TARGET: &str = "handle_grip_site";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionRecord {
    pub waypoint_id: String,
    pub target: String,
    pub artifact_uri: String,
    pub anomaly_detected: bool,
}

/// Adapter boundary for MuJoCo rendering or a future perception service.
pub trait W1InspectionObserver {
    fn waypoint_reached(&self, waypoint_id: &str) -> eyre::Result<bool>;
    fn capture(&mut self, waypoint_id: &str, target: &str) -> eyre::Result<InspectionRecord>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerState {
    Idle,
    WaitingForWaypoint(usize),
    Completed,
    Aborted,
}

struct InspectionWaypoint {
    id: &'static str,
    target: &'static str,
    commands: Vec<ActuatorCommand>,
}

pub struct W1InspectRunner<T: ActuatorTransport, O: W1InspectionObserver> {
    authority: CommandAuthority,
    principal: AuthContext,
    shim: KosSafetyShim<T>,
    observer: O,
    next_sequence: u64,
    state: RunnerState,
    records: Vec<InspectionRecord>,
}

impl<T: ActuatorTransport, O: W1InspectionObserver> W1InspectRunner<T, O> {
    pub fn new(
        authority: CommandAuthority,
        principal: AuthContext,
        shim: KosSafetyShim<T>,
        observer: O,
        initial_sequence: u64,
    ) -> eyre::Result<Self> {
        if !principal.is_authenticated() {
            return Err(eyre::eyre!(
                "W1 inspection runner requires an authenticated principal"
            ));
        }
        if principal.role() == Role::Observer {
            return Err(eyre::eyre!(
                "W1 inspection runner requires Operator or Supervisor role"
            ));
        }
        Ok(Self {
            authority,
            principal,
            shim,
            observer,
            next_sequence: initial_sequence,
            state: RunnerState::Idle,
            records: Vec::with_capacity(2),
        })
    }

    pub fn records(&self) -> &[InspectionRecord] {
        &self.records
    }

    pub fn observer(&self) -> &O {
        &self.observer
    }

    fn command_waypoint(&mut self, step: &PlanStep, index: usize) -> eyre::Result<()> {
        let waypoint = waypoints()
            .into_iter()
            .nth(index)
            .ok_or_else(|| eyre::eyre!("unknown W1 inspection waypoint {index}"))?;
        let command_id = format!("{}-{}-{}", step.id, waypoint.id, self.next_sequence);
        let decision = self.authority.authorize(
            &self.principal,
            CommandRequest {
                id: command_id,
                sequence: self.next_sequence,
                class: CommandClass::Queued,
                action: format!(
                    "move to W1 inspection waypoint '{}' for target '{}'",
                    waypoint.id, waypoint.target
                ),
                approval_token: None,
            },
        )?;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| eyre::eyre!("inspection command sequence exhausted"))?;
        self.shim.command(decision, waypoint.commands)?;
        self.state = RunnerState::WaitingForWaypoint(index);
        Ok(())
    }

    fn enter_safe_state(&mut self, step: &PlanStep, reason: &str) -> eyre::Result<()> {
        let decision = self.authority.authorize(
            &self.principal,
            CommandRequest {
                id: format!("{}-abort", step.id),
                sequence: self.next_sequence,
                class: CommandClass::Immediate,
                action: format!("abort W1 inspection: {reason}"),
                approval_token: None,
            },
        )?;
        self.shim.enter_safe_authorized(decision, reason)?;
        self.state = RunnerState::Aborted;
        Ok(())
    }
}

impl<T: ActuatorTransport, O: W1InspectionObserver> SkillRunner for W1InspectRunner<T, O> {
    fn start(&mut self, step: &PlanStep) -> eyre::Result<()> {
        if step.skill != SKILL_NAME {
            return Err(eyre::eyre!(
                "W1 inspection runner cannot execute skill '{}'",
                step.skill
            ));
        }
        if step.requested_level > AutonomyLevel::L2 {
            return Err(eyre::eyre!("W1 inspection runner is capped at L2"));
        }
        if matches!(self.state, RunnerState::WaitingForWaypoint(_)) {
            return Err(eyre::eyre!("W1 inspection is already active"));
        }
        self.records.clear();
        self.state = RunnerState::Idle;
        self.command_waypoint(step, 0)
    }

    fn poll(&mut self, step: &PlanStep) -> eyre::Result<SkillPoll> {
        let RunnerState::WaitingForWaypoint(index) = self.state else {
            return match self.state {
                RunnerState::Completed => Ok(SkillPoll::Succeeded),
                RunnerState::Aborted => Ok(SkillPoll::Failed("inspection aborted".to_owned())),
                RunnerState::Idle | RunnerState::WaitingForWaypoint(_) => {
                    Err(eyre::eyre!("W1 inspection has not started"))
                }
            };
        };
        let waypoint = waypoints()
            .into_iter()
            .nth(index)
            .ok_or_else(|| eyre::eyre!("unknown W1 inspection waypoint {index}"))?;
        if !self.observer.waypoint_reached(waypoint.id)? {
            return Ok(SkillPoll::Running);
        }
        let record = self.observer.capture(waypoint.id, waypoint.target)?;
        if record.waypoint_id != waypoint.id || record.target != waypoint.target {
            return Err(eyre::eyre!(
                "observer record does not match requested W1 target"
            ));
        }
        if record.artifact_uri.trim().is_empty() {
            return Err(eyre::eyre!(
                "observer returned an empty artifact URI for W1 target '{}'",
                waypoint.target
            ));
        }
        self.records.push(record);
        let next = index + 1;
        if next == waypoints().len() {
            self.state = RunnerState::Completed;
            Ok(SkillPoll::Succeeded)
        } else {
            self.command_waypoint(step, next)?;
            Ok(SkillPoll::Running)
        }
    }

    fn abort(&mut self, step: &PlanStep, reason: &str) -> eyre::Result<()> {
        self.enter_safe_state(step, reason)
    }
}

fn command(actuator_id: u32, position_deg: f64, max_torque_nm: f64) -> ActuatorCommand {
    ActuatorCommand {
        actuator_id,
        position_deg: Some(position_deg),
        velocity_deg_s: Some(30.0),
        torque_nm: Some(max_torque_nm),
    }
}

fn waypoints() -> [InspectionWaypoint; 2] {
    [
        InspectionWaypoint {
            id: "panel-overview",
            target: PANEL_TARGET,
            commands: vec![
                command(11, 35.0, 8.0),
                command(12, 0.0, 8.0),
                command(14, -55.0, 4.0),
                command(21, -35.0, 8.0),
                command(22, 0.0, 8.0),
                command(24, 55.0, 4.0),
            ],
        },
        InspectionWaypoint {
            id: "handle-detail",
            target: HANDLE_TARGET,
            commands: vec![
                command(11, 50.0, 8.0),
                command(12, -10.0, 8.0),
                command(14, -70.0, 4.0),
                command(21, -25.0, 8.0),
                command(22, 10.0, 8.0),
                command(24, 45.0, 4.0),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inspect_fixture_spec, Condition, FactSet, PlanExecutor, TaskPlan};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::Duration;
    use trion_runtime::{
        CommandGate, EventKind, EventLog, ModeController, RobotManifest, Role, RuntimeClock,
    };

    const MANIFEST: &str = include_str!("../../../config/robot_manifest.yaml");
    const W1: &str = include_str!("../../../sim/worksites/w1_panel.xml");

    #[derive(Default)]
    struct TransportState {
        forwarded: Vec<Vec<ActuatorCommand>>,
        safe_ids: Vec<u32>,
    }

    #[derive(Clone, Default)]
    struct TestTransport(Arc<Mutex<TransportState>>);

    impl TestTransport {
        fn state(&self) -> MutexGuard<'_, TransportState> {
            match self.0.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }
        }
    }

    impl ActuatorTransport for TestTransport {
        fn forward(&self, commands: &[ActuatorCommand]) -> eyre::Result<()> {
            self.state().forwarded.push(commands.to_vec());
            Ok(())
        }

        fn command_safe_state(&self, actuator_ids: &[u32]) -> eyre::Result<()> {
            self.state().safe_ids = actuator_ids.to_vec();
            Ok(())
        }
    }

    #[derive(Default)]
    struct TestObserver {
        reached: bool,
        captures: Vec<String>,
    }

    impl W1InspectionObserver for TestObserver {
        fn waypoint_reached(&self, _waypoint_id: &str) -> eyre::Result<bool> {
            Ok(self.reached)
        }

        fn capture(&mut self, waypoint_id: &str, target: &str) -> eyre::Result<InspectionRecord> {
            self.captures.push(target.to_owned());
            Ok(InspectionRecord {
                waypoint_id: waypoint_id.to_owned(),
                target: target.to_owned(),
                artifact_uri: format!("memory://{target}.png"),
                anomaly_detected: false,
            })
        }
    }

    fn plan() -> eyre::Result<TaskPlan> {
        TaskPlan::from_json(include_str!("../../../plans/w1_inspect_and_handle.json")).map(
            |mut plan| {
                plan.steps.truncate(1);
                plan
            },
        )
    }

    fn runner(
        events: EventLog,
    ) -> eyre::Result<(W1InspectRunner<TestTransport, TestObserver>, TestTransport)> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let ids = manifest
            .robot
            .actuators
            .iter()
            .map(|actuator| actuator.id)
            .collect();
        let transport = TestTransport::default();
        let shim = KosSafetyShim::new(
            transport.clone(),
            CommandGate::new(&manifest.robot.actuators)?,
            ModeController::new(events.clone()),
            events.clone(),
            ids,
            Duration::from_millis(100),
        )?;
        let authority =
            CommandAuthority::new(RuntimeClock::new(), events, Duration::from_secs(5), 4, 4)?;
        let principal = AuthContext::authenticated("inspection-operator", Role::Operator)?;
        Ok((
            W1InspectRunner::new(
                authority,
                principal,
                shim,
                TestObserver {
                    reached: true,
                    ..TestObserver::default()
                },
                1,
            )?,
            transport,
        ))
    }

    fn facts(active_fault: bool) -> FactSet {
        FactSet::from([
            ("perception_nominal".to_owned(), true),
            ("localization_valid".to_owned(), true),
            ("active_fault".to_owned(), active_fault),
        ])
    }

    #[test]
    fn w1_targets_and_actuator_profile_match_checked_in_assets() -> eyre::Result<()> {
        assert!(W1.contains("name=\"panel\""));
        assert!(W1.contains("name=\"handle_grip_site\""));
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let gate = CommandGate::new(&manifest.robot.actuators)?;
        for waypoint in waypoints() {
            for actuator_command in waypoint.commands {
                assert!(!gate.validate(actuator_command)?.limited);
            }
        }
        Ok(())
    }

    #[test]
    fn plan_executes_w1_inspection_through_authority_and_shim() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 128)?;
        let (mut runner, transport) = runner(events.clone())?;
        let mut executor = PlanExecutor::new(clock, events.clone(), 4, 2)?;
        executor.register_skill(inspect_fixture_spec())?;
        executor.start_plan(plan()?)?;

        assert!(matches!(
            executor.tick(&facts(false), &mut runner)?,
            crate::ExecutorStatus::Executing { .. }
        ));
        assert!(matches!(
            executor.tick(&facts(false), &mut runner)?,
            crate::ExecutorStatus::Executing { .. }
        ));
        assert!(matches!(
            executor.tick(&facts(false), &mut runner)?,
            crate::ExecutorStatus::Completed { .. }
        ));

        assert_eq!(transport.state().forwarded.len(), 2);
        assert_eq!(runner.records().len(), 2);
        assert_eq!(runner.observer().captures, [PANEL_TARGET, HANDLE_TARGET]);
        let kinds: Vec<_> = events
            .snapshot()
            .into_iter()
            .map(|record| record.kind)
            .collect();
        assert!(kinds.contains(&EventKind::CommandAuthorized));
        assert!(kinds.contains(&EventKind::CommandForwarded));
        assert!(kinds.contains(&EventKind::PlanCompleted));
        Ok(())
    }

    #[test]
    fn plan_fault_aborts_runner_through_immediate_safe_state() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 128)?;
        let (mut runner, transport) = runner(events.clone())?;
        let mut executor = PlanExecutor::new(clock, events.clone(), 4, 2)?;
        executor.register_skill(inspect_fixture_spec())?;
        executor.start_plan(plan()?)?;
        executor.tick(&facts(false), &mut runner)?;
        assert!(matches!(
            executor.tick(&facts(true), &mut runner)?,
            crate::ExecutorStatus::Aborted { .. }
        ));
        assert_eq!(transport.state().safe_ids.len(), 20);
        assert!(events
            .snapshot()
            .iter()
            .any(|record| record.kind == EventKind::SafeStateCommanded));
        Ok(())
    }

    #[test]
    fn inverted_required_precondition_is_not_accepted() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 32)?;
        let mut executor = PlanExecutor::new(clock, events, 4, 2)?;
        executor.register_skill(inspect_fixture_spec())?;
        let mut invalid = plan()?;
        invalid.steps[0].preconditions = vec![
            Condition {
                key: "perception_nominal".to_owned(),
                expected: false,
            },
            Condition {
                key: "localization_valid".to_owned(),
                expected: true,
            },
        ];
        assert!(executor.validate_plan(&invalid).is_err());
        Ok(())
    }
}
