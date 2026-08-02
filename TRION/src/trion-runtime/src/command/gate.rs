//! Independent actuator command sanity gate (TR-P4-020, F-09).

use crate::command::safe_mode::RunMode;
use crate::config::ActuatorConfig;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ActuatorCommand {
    pub actuator_id: u32,
    pub position_deg: Option<f64>,
    pub velocity_deg_s: Option<f64>,
    pub torque_nm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GateOutcome {
    pub command: ActuatorCommand,
    pub limited: bool,
}

pub struct CommandGate {
    config: HashMap<u32, ActuatorConfig>,
}

impl CommandGate {
    pub fn new(actuators: &[ActuatorConfig]) -> eyre::Result<Self> {
        let mut config = HashMap::with_capacity(actuators.len());
        for actuator in actuators {
            if config.insert(actuator.id, actuator.clone()).is_some() {
                return Err(eyre::eyre!("duplicate actuator id {}", actuator.id));
            }
        }
        if config.is_empty() {
            return Err(eyre::eyre!("command gate requires actuator configuration"));
        }
        Ok(Self { config })
    }

    pub fn validate(&self, command: ActuatorCommand) -> eyre::Result<GateOutcome> {
        let actuator = self
            .config
            .get(&command.actuator_id)
            .ok_or_else(|| eyre::eyre!("unknown actuator {}", command.actuator_id))?;

        for value in [
            command.position_deg,
            command.velocity_deg_s,
            command.torque_nm,
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() {
                return Err(eyre::eyre!(
                    "actuator {} command contains a non-finite value",
                    command.actuator_id
                ));
            }
        }

        let position_deg = command
            .position_deg
            .map(|value| value.clamp(actuator.limit_min_deg, actuator.limit_max_deg));
        let velocity_deg_s = command
            .velocity_deg_s
            .map(|value| value.clamp(-actuator.max_velocity_deg_s, actuator.max_velocity_deg_s));
        let torque_nm = command
            .torque_nm
            .map(|value| value.clamp(-actuator.max_torque_nm, actuator.max_torque_nm));
        let sanitized = ActuatorCommand {
            actuator_id: command.actuator_id,
            position_deg,
            velocity_deg_s,
            torque_nm,
        };
        let limited = sanitized != command;
        Ok(GateOutcome {
            command: sanitized,
            limited,
        })
    }

    /// Reject every ordinary actuator command while safe-mode is active.
    /// The future KOS shim is responsible for issuing the separate brake or
    /// zero-torque safe-state command on the privileged recovery path.
    pub fn validate_for_mode(
        &self,
        mode: RunMode,
        command: ActuatorCommand,
    ) -> eyre::Result<GateOutcome> {
        if mode == RunMode::Safe {
            return Err(eyre::eyre!(
                "actuator command rejected: safe-mode is active"
            ));
        }
        self.validate(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RobotManifest;

    const MANIFEST: &str = include_str!("../../../../config/robot_manifest.yaml");

    #[test]
    fn command_is_clamped_to_manifest_limits() -> eyre::Result<()> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let gate = CommandGate::new(&manifest.robot.actuators)?;
        let outcome = gate.validate(ActuatorCommand {
            actuator_id: 11,
            position_deg: Some(200.0),
            velocity_deg_s: Some(-500.0),
            torque_nm: Some(100.0),
        })?;
        assert!(outcome.limited);
        assert_eq!(outcome.command.position_deg, Some(180.0));
        assert_eq!(outcome.command.velocity_deg_s, Some(-360.0));
        assert_eq!(outcome.command.torque_nm, Some(40.0));
        Ok(())
    }

    #[test]
    fn unknown_and_non_finite_commands_are_rejected() -> eyre::Result<()> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let gate = CommandGate::new(&manifest.robot.actuators)?;
        let unknown = ActuatorCommand {
            actuator_id: 999,
            position_deg: Some(0.0),
            velocity_deg_s: None,
            torque_nm: None,
        };
        assert!(gate.validate(unknown).is_err());
        let invalid = ActuatorCommand {
            actuator_id: 11,
            position_deg: Some(f64::NAN),
            velocity_deg_s: None,
            torque_nm: None,
        };
        assert!(gate.validate(invalid).is_err());
        Ok(())
    }

    #[test]
    fn safe_mode_rejects_actuator_commands() -> eyre::Result<()> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        let gate = CommandGate::new(&manifest.robot.actuators)?;
        let command = ActuatorCommand {
            actuator_id: 11,
            position_deg: Some(90.0),
            velocity_deg_s: Some(0.0),
            torque_nm: Some(0.0),
        };
        assert!(gate.validate_for_mode(RunMode::Safe, command).is_err());
        Ok(())
    }
}
