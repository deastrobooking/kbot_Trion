//! Supervised-autonomy skills and bounded task-plan execution (P1/P3).

pub mod executor;
pub mod model;
pub mod skills;
pub mod w1_inspect;

pub use executor::{ExecutorStatus, FactSet, PlanExecutor, SkillPoll, SkillRunner};
pub use model::{AutonomyLevel, Condition, HumanGate, PlanStep, SkillSpec, TaskPlan};
pub use skills::{actuate_handle_spec, inspect_fixture_spec, mate_connector_spec};
pub use w1_inspect::{InspectionRecord, W1InspectRunner, W1InspectionObserver};
