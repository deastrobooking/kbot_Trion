//! Initial canonical skill specifications. These define execution envelopes,
//! not motion implementations or promotion evidence.

use crate::model::{AutonomyLevel, Condition, SkillSpec};

fn condition(key: &str, expected: bool) -> Condition {
    Condition {
        key: key.to_owned(),
        expected,
    }
}

pub fn inspect_fixture_spec() -> SkillSpec {
    SkillSpec {
        name: "inspect_fixture".to_owned(),
        max_level: AutonomyLevel::L2,
        required_preconditions: vec![
            condition("perception_nominal", true),
            condition("localization_valid", true),
        ],
        required_abort_conditions: vec![condition("active_fault", true)],
    }
}

pub fn actuate_handle_spec() -> SkillSpec {
    SkillSpec {
        name: "actuate_handle".to_owned(),
        max_level: AutonomyLevel::L2,
        required_preconditions: vec![
            condition("handle_localized", true),
            condition("arm_health_nominal", true),
        ],
        required_abort_conditions: vec![
            condition("active_fault", true),
            condition("force_limit_exceeded", true),
        ],
    }
}

pub fn mate_connector_spec() -> SkillSpec {
    SkillSpec {
        name: "mate_connector".to_owned(),
        max_level: AutonomyLevel::L1,
        required_preconditions: vec![
            condition("connector_halves_localized", true),
            condition("force_sensing_available", true),
        ],
        required_abort_conditions: vec![
            condition("active_fault", true),
            condition("insertion_force_exceeded", true),
        ],
    }
}
