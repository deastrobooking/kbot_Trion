//! Declarative, serializable skill and task-plan model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AutonomyLevel {
    L0,
    L1,
    L2,
    L3,
    L4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub key: String,
    pub expected: bool,
}

impl Condition {
    pub fn is_satisfied(&self, facts: &std::collections::HashMap<String, bool>) -> bool {
        facts.get(&self.key).copied() == Some(self.expected)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanGate {
    pub id: String,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSpec {
    pub name: String,
    pub max_level: AutonomyLevel,
    pub required_preconditions: Vec<Condition>,
    pub required_abort_conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub skill: String,
    pub requested_level: AutonomyLevel,
    pub timeout_ms: u64,
    pub preconditions: Vec<Condition>,
    pub abort_conditions: Vec<Condition>,
    pub human_gate: Option<HumanGate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub steps: Vec<PlanStep>,
}

impl TaskPlan {
    /// Decode a task-plan artifact. Execution-envelope validation remains the
    /// responsibility of [`crate::PlanExecutor::validate_plan`].
    pub fn from_json(input: &str) -> eyre::Result<Self> {
        serde_json::from_str(input).map_err(|error| eyre::eyre!("invalid task-plan JSON: {error}"))
    }

    /// Encode a task plan in the canonical, reviewable artifact format.
    pub fn to_json_pretty(&self) -> eyre::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| eyre::eyre!("failed to encode task-plan JSON: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::TaskPlan;

    #[test]
    fn checked_in_plan_round_trips_through_canonical_json() -> eyre::Result<()> {
        let source = include_str!("../../../plans/w1_inspect_and_handle.json");
        let plan = TaskPlan::from_json(source)?;
        let encoded = plan.to_json_pretty()?;
        assert_eq!(TaskPlan::from_json(&encoded)?, plan);
        Ok(())
    }
}
