//! Safety boundary in front of an actuator transport (TR-P4-020, F-09).

use crate::command::gate::{ActuatorCommand, CommandGate, GateOutcome};
use crate::command::safe_mode::ModeController;
use crate::telemetry::events::{EventKind, EventLog};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub trait ActuatorTransport: Send + Sync + 'static {
    fn forward(&self, commands: &[ActuatorCommand]) -> eyre::Result<()>;
    fn command_safe_state(&self, actuator_ids: &[u32]) -> eyre::Result<()>;
}

pub trait SafeStateAction: Send + Sync + 'static {
    fn enter_safe_state(&self, reason: &str) -> eyre::Result<Duration>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SafeStateReceipt {
    pub actuator_count: usize,
    pub elapsed: Duration,
}

pub struct KosSafetyShim<T: ActuatorTransport> {
    transport: T,
    gate: CommandGate,
    modes: ModeController,
    events: EventLog,
    actuator_ids: Vec<u32>,
    operation_lock: Mutex<()>,
    safe_state_deadline: Duration,
}

impl<T: ActuatorTransport> KosSafetyShim<T> {
    pub fn new(
        transport: T,
        gate: CommandGate,
        modes: ModeController,
        events: EventLog,
        mut actuator_ids: Vec<u32>,
        safe_state_deadline: Duration,
    ) -> eyre::Result<Self> {
        if actuator_ids.is_empty() {
            return Err(eyre::eyre!("safety shim requires at least one actuator"));
        }
        if safe_state_deadline.is_zero() {
            return Err(eyre::eyre!("safe-state deadline must be greater than zero"));
        }
        actuator_ids.sort_unstable();
        actuator_ids.dedup();
        Ok(Self {
            transport,
            gate,
            modes,
            events,
            actuator_ids,
            operation_lock: Mutex::new(()),
            safe_state_deadline,
        })
    }

    fn lock_operations(&self) -> MutexGuard<'_, ()> {
        match self.operation_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub fn command(&self, commands: Vec<ActuatorCommand>) -> eyre::Result<Vec<GateOutcome>> {
        let _guard = self.lock_operations();
        if self.modes.is_safe() {
            self.events.record(
                EventKind::CommandRejected,
                None,
                "actuator command rejected: safe-mode is active",
            );
            return Err(eyre::eyre!(
                "actuator command rejected: safe-mode is active"
            ));
        }
        if commands.is_empty() {
            return Err(eyre::eyre!("actuator command batch must not be empty"));
        }

        let mut outcomes = Vec::with_capacity(commands.len());
        for command in commands {
            let outcome = self.gate.validate_for_mode(self.modes.mode(), command)?;
            if outcome.limited {
                self.events.record(
                    EventKind::CommandLimited,
                    Some(&outcome.command.actuator_id.to_string()),
                    "actuator command clamped to manifest limits",
                );
            }
            outcomes.push(outcome);
        }
        let sanitized: Vec<_> = outcomes
            .iter()
            .map(|outcome| outcome.command.clone())
            .collect();
        self.transport.forward(&sanitized)?;
        Ok(outcomes)
    }

    pub fn enter_safe(&self, reason: &str) -> eyre::Result<SafeStateReceipt> {
        let _guard = self.lock_operations();
        let started = Instant::now();
        self.modes.enter_safe(reason);
        self.transport.command_safe_state(&self.actuator_ids)?;
        let elapsed = started.elapsed();
        self.events.record(
            EventKind::SafeStateCommanded,
            None,
            format!(
                "safe state commanded for {} actuators in {} us",
                self.actuator_ids.len(),
                elapsed.as_micros()
            ),
        );
        if elapsed > self.safe_state_deadline {
            return Err(eyre::eyre!(
                "safe-state deadline exceeded: {} us > {} us",
                elapsed.as_micros(),
                self.safe_state_deadline.as_micros()
            ));
        }
        Ok(SafeStateReceipt {
            actuator_count: self.actuator_ids.len(),
            elapsed,
        })
    }
}

impl<T: ActuatorTransport> SafeStateAction for KosSafetyShim<T> {
    fn enter_safe_state(&self, reason: &str) -> eyre::Result<Duration> {
        self.enter_safe(reason).map(|receipt| receipt.elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RobotManifest;
    use crate::time::RuntimeClock;
    use std::collections::HashMap;
    use std::sync::Arc;

    const MANIFEST: &str = include_str!("../../../../config/robot_manifest.yaml");

    #[derive(Default)]
    struct TransportState {
        forwarded: Vec<ActuatorCommand>,
        safe_ids: Vec<u32>,
        torque_nm: HashMap<u32, f64>,
    }

    #[derive(Clone, Default)]
    struct SimTransport(Arc<Mutex<TransportState>>);

    impl SimTransport {
        fn state(&self) -> MutexGuard<'_, TransportState> {
            match self.0.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            }
        }
    }

    impl ActuatorTransport for SimTransport {
        fn forward(&self, commands: &[ActuatorCommand]) -> eyre::Result<()> {
            let mut state = self.state();
            state.forwarded.extend_from_slice(commands);
            for command in commands {
                state
                    .torque_nm
                    .insert(command.actuator_id, command.torque_nm.unwrap_or(0.0));
            }
            Ok(())
        }

        fn command_safe_state(&self, actuator_ids: &[u32]) -> eyre::Result<()> {
            let mut state = self.state();
            state.safe_ids = actuator_ids.to_vec();
            for actuator_id in actuator_ids {
                state.torque_nm.insert(*actuator_id, 0.0);
            }
            Ok(())
        }
    }

    #[test]
    fn safe_state_is_bounded_and_blocks_later_commands() -> eyre::Result<()> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let ids = manifest
            .robot
            .actuators
            .iter()
            .map(|item| item.id)
            .collect();
        let gate = CommandGate::new(&manifest.robot.actuators)?;
        let events = EventLog::new(RuntimeClock::new(), 32)?;
        let modes = ModeController::new(events.clone());
        let transport = SimTransport::default();
        let shim = KosSafetyShim::new(
            transport.clone(),
            gate,
            modes,
            events.clone(),
            ids,
            Duration::from_millis(100),
        )?;

        let outcome = shim.command(vec![ActuatorCommand {
            actuator_id: 11,
            position_deg: Some(200.0),
            velocity_deg_s: Some(0.0),
            torque_nm: Some(10.0),
        }])?;
        assert!(outcome[0].limited);
        assert_eq!(transport.state().forwarded[0].position_deg, Some(180.0));

        let receipt = shim.enter_safe("injected actuator fault")?;
        assert_eq!(receipt.actuator_count, 20);
        assert!(receipt.elapsed <= Duration::from_millis(100));
        assert_eq!(transport.state().safe_ids.len(), 20);
        assert!(transport
            .state()
            .torque_nm
            .values()
            .all(|torque| *torque == 0.0));
        assert!(shim
            .command(vec![ActuatorCommand {
                actuator_id: 11,
                position_deg: Some(0.0),
                velocity_deg_s: None,
                torque_nm: None,
            }])
            .is_err());
        let log = events.to_jsonl();
        assert!(log.contains("SafeStateCommanded"));
        assert!(log.contains("CommandRejected"));
        Ok(())
    }
}
