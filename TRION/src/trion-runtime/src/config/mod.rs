//! Versioned robot configuration manifest and startup validation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotManifest {
    pub manifest_version: String,
    pub hardware_approved: bool,
    pub robot: RobotConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfig {
    pub serial: String,
    pub model: String,
    pub actuators: Vec<ActuatorConfig>,
    pub imus: Vec<ImuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorConfig {
    pub id: u32,
    pub name: String,
    pub actuator_type: String,
    pub bus: String,
    pub limit_min_deg: f64,
    pub limit_max_deg: f64,
    pub max_velocity_deg_s: f64,
    pub max_torque_nm: f64,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuConfig {
    pub name: String,
    pub driver: String,
    pub interface: String,
    pub baud_rate: u32,
    pub frequency_hz: u32,
    pub critical: bool,
}

impl RobotManifest {
    pub fn from_yaml(yaml: &str) -> eyre::Result<Self> {
        let manifest: Self = serde_yaml_ng::from_str(yaml)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> eyre::Result<()> {
        if self.manifest_version.trim().is_empty() {
            return Err(eyre::eyre!("manifest version must not be empty"));
        }
        if self.robot.serial.trim().is_empty() || self.robot.model.trim().is_empty() {
            return Err(eyre::eyre!("robot serial and model must not be empty"));
        }
        if self.robot.actuators.is_empty() {
            return Err(eyre::eyre!("manifest must define at least one actuator"));
        }

        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for actuator in &self.robot.actuators {
            if !ids.insert(actuator.id) {
                return Err(eyre::eyre!("duplicate actuator id {}", actuator.id));
            }
            if actuator.name.trim().is_empty() || !names.insert(actuator.name.as_str()) {
                return Err(eyre::eyre!(
                    "empty or duplicate actuator name '{}'",
                    actuator.name
                ));
            }
            if actuator.bus.trim().is_empty() || actuator.actuator_type.trim().is_empty() {
                return Err(eyre::eyre!(
                    "actuator {} must define its type and bus",
                    actuator.id
                ));
            }
            let limits = [
                actuator.limit_min_deg,
                actuator.limit_max_deg,
                actuator.max_velocity_deg_s,
                actuator.max_torque_nm,
            ];
            if limits.iter().any(|value| !value.is_finite()) {
                return Err(eyre::eyre!(
                    "actuator {} has a non-finite limit",
                    actuator.id
                ));
            }
            if actuator.limit_min_deg >= actuator.limit_max_deg
                || actuator.max_velocity_deg_s <= 0.0
                || actuator.max_torque_nm <= 0.0
            {
                return Err(eyre::eyre!("actuator {} has invalid limits", actuator.id));
            }
        }
        Ok(())
    }

    pub fn require_hardware_approval(&self) -> eyre::Result<()> {
        if !self.hardware_approved {
            return Err(eyre::eyre!(
                "robot manifest is simulation-only; hardware limits are not approved"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../../../../config/robot_manifest.yaml");

    #[test]
    fn checked_in_manifest_is_valid_and_simulation_only() -> eyre::Result<()> {
        let manifest = RobotManifest::from_yaml(MANIFEST)?;
        assert_eq!(manifest.robot.actuators.len(), 20);
        assert!(manifest.require_hardware_approval().is_err());
        Ok(())
    }

    #[test]
    fn duplicate_actuator_ids_are_rejected() {
        let yaml = MANIFEST.replacen("id: 12", "id: 11", 1);
        assert!(RobotManifest::from_yaml(&yaml).is_err());
    }
}
