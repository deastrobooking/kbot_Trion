//! Deterministic bounded task-plan executor for L2/L3 supervised autonomy.

use crate::model::{AutonomyLevel, PlanStep, SkillSpec, TaskPlan};
use std::collections::{HashMap, HashSet};
use trion_runtime::{AuthContext, EventKind, EventLog, Role, RuntimeClock};

pub type FactSet = HashMap<String, bool>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillPoll {
    Running,
    Succeeded,
    Failed(String),
}

pub trait SkillRunner {
    fn start(&mut self, step: &PlanStep) -> eyre::Result<()>;
    fn poll(&mut self, step: &PlanStep) -> eyre::Result<SkillPoll>;
    fn abort(&mut self, step: &PlanStep, reason: &str) -> eyre::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutorStatus {
    Idle,
    WaitingPreconditions {
        step_id: String,
        missing: Vec<String>,
    },
    AwaitingGate {
        step_id: String,
        gate_id: String,
    },
    Executing {
        step_id: String,
    },
    StepCompleted {
        step_id: String,
    },
    Completed {
        plan_id: String,
    },
    Aborted {
        plan_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepPhase {
    Ready,
    AwaitingGate,
    Executing,
}

struct ActivePlan {
    plan: TaskPlan,
    step_index: usize,
    step_activated_ms: u64,
    phase: StepPhase,
    approved_gate: Option<String>,
}

pub struct PlanExecutor {
    clock: RuntimeClock,
    events: EventLog,
    max_steps: usize,
    skill_capacity: usize,
    skills: HashMap<String, SkillSpec>,
    active: Option<ActivePlan>,
    terminal: Option<ExecutorStatus>,
}

impl PlanExecutor {
    pub fn new(
        clock: RuntimeClock,
        events: EventLog,
        max_steps: usize,
        skill_capacity: usize,
    ) -> eyre::Result<Self> {
        if max_steps == 0 || skill_capacity == 0 {
            return Err(eyre::eyre!("executor capacities must be greater than zero"));
        }
        Ok(Self {
            clock,
            events,
            max_steps,
            skill_capacity,
            skills: HashMap::with_capacity(skill_capacity),
            active: None,
            terminal: None,
        })
    }

    pub fn register_skill(&mut self, spec: SkillSpec) -> eyre::Result<()> {
        self.validate_skill_spec(&spec)?;
        if self.skills.contains_key(&spec.name) {
            return Err(eyre::eyre!("skill '{}' is already registered", spec.name));
        }
        if self.skills.len() >= self.skill_capacity {
            return Err(eyre::eyre!("skill registry capacity exhausted"));
        }
        self.skills.insert(spec.name.clone(), spec);
        Ok(())
    }

    pub fn validate_plan(&self, plan: &TaskPlan) -> eyre::Result<()> {
        if plan.id.trim().is_empty() {
            return Err(eyre::eyre!("plan id must not be empty"));
        }
        if plan.steps.is_empty() || plan.steps.len() > self.max_steps {
            return Err(eyre::eyre!(
                "plan must contain between 1 and {} steps",
                self.max_steps
            ));
        }
        let mut step_ids = HashSet::with_capacity(plan.steps.len());
        for step in &plan.steps {
            if step.id.trim().is_empty() || !step_ids.insert(step.id.as_str()) {
                return Err(eyre::eyre!("plan contains an empty or duplicate step id"));
            }
            if step.timeout_ms == 0 {
                return Err(eyre::eyre!("step '{}' has zero timeout", step.id));
            }
            let spec = self
                .skills
                .get(&step.skill)
                .ok_or_else(|| eyre::eyre!("step '{}' uses unknown skill", step.id))?;
            if step.requested_level > spec.max_level {
                return Err(eyre::eyre!(
                    "step '{}' requests {:?} above skill maximum {:?}",
                    step.id,
                    step.requested_level,
                    spec.max_level
                ));
            }
            if step.requested_level == AutonomyLevel::L3 && step.human_gate.is_none() {
                return Err(eyre::eyre!("L3 step '{}' requires a human gate", step.id));
            }
            if let Some(gate) = &step.human_gate {
                if gate.id.trim().is_empty() || gate.prompt.trim().is_empty() {
                    return Err(eyre::eyre!("step '{}' has an invalid human gate", step.id));
                }
            }
            self.validate_conditions(step, spec)?;
        }
        Ok(())
    }

    pub fn start_plan(&mut self, plan: TaskPlan) -> eyre::Result<()> {
        self.start_plan_at(plan, self.clock.now_ms())
    }

    fn start_plan_at(&mut self, plan: TaskPlan, now_ms: u64) -> eyre::Result<()> {
        if self.active.is_some() {
            return Err(eyre::eyre!("another plan is already active"));
        }
        self.validate_plan(&plan)?;
        self.events.record(
            EventKind::PlanStarted,
            Some(&plan.id),
            format!("plan started with {} steps", plan.steps.len()),
        );
        self.terminal = None;
        self.active = Some(ActivePlan {
            plan,
            step_index: 0,
            step_activated_ms: now_ms,
            phase: StepPhase::Ready,
            approved_gate: None,
        });
        Ok(())
    }

    pub fn tick<R: SkillRunner>(
        &mut self,
        facts: &FactSet,
        runner: &mut R,
    ) -> eyre::Result<ExecutorStatus> {
        self.tick_at(self.clock.now_ms(), facts, runner)
    }

    fn tick_at<R: SkillRunner>(
        &mut self,
        now_ms: u64,
        facts: &FactSet,
        runner: &mut R,
    ) -> eyre::Result<ExecutorStatus> {
        let Some(mut active) = self.active.take() else {
            return Ok(self.terminal.clone().unwrap_or(ExecutorStatus::Idle));
        };
        let step = active.plan.steps[active.step_index].clone();

        if let Some(condition) = step
            .abort_conditions
            .iter()
            .find(|condition| condition.is_satisfied(facts))
        {
            let reason = format!("abort condition '{}' triggered", condition.key);
            return self.finish_aborted(active, &step, reason, runner);
        }
        if now_ms.saturating_sub(active.step_activated_ms) > step.timeout_ms {
            let reason = format!("step '{}' exceeded {} ms timeout", step.id, step.timeout_ms);
            return self.finish_aborted(active, &step, reason, runner);
        }

        match active.phase {
            StepPhase::Ready => {
                let missing: Vec<_> = step
                    .preconditions
                    .iter()
                    .filter(|condition| !condition.is_satisfied(facts))
                    .map(|condition| condition.key.clone())
                    .collect();
                if !missing.is_empty() {
                    let status = ExecutorStatus::WaitingPreconditions {
                        step_id: step.id,
                        missing,
                    };
                    self.active = Some(active);
                    return Ok(status);
                }

                if let Some(gate) = &step.human_gate {
                    if active.approved_gate.as_deref() != Some(gate.id.as_str()) {
                        active.phase = StepPhase::AwaitingGate;
                        self.events.record(
                            EventKind::HumanGateRequested,
                            Some(&step.skill),
                            format!("gate '{}' requested for step '{}'", gate.id, step.id),
                        );
                        let status = ExecutorStatus::AwaitingGate {
                            step_id: step.id,
                            gate_id: gate.id.clone(),
                        };
                        self.active = Some(active);
                        return Ok(status);
                    }
                }

                if let Err(error) = runner.start(&step) {
                    let reason = format!("skill '{}' failed to start: {error}", step.skill);
                    return self.finish_aborted(active, &step, reason, runner);
                }
                active.phase = StepPhase::Executing;
                self.events.record(
                    EventKind::SkillStarted,
                    Some(&step.skill),
                    format!("step '{}' started at {:?}", step.id, step.requested_level),
                );
                let status = ExecutorStatus::Executing { step_id: step.id };
                self.active = Some(active);
                Ok(status)
            }
            StepPhase::AwaitingGate => {
                let gate = step
                    .human_gate
                    .as_ref()
                    .ok_or_else(|| eyre::eyre!("executor gate state without gate spec"))?;
                let status = ExecutorStatus::AwaitingGate {
                    step_id: step.id,
                    gate_id: gate.id.clone(),
                };
                self.active = Some(active);
                Ok(status)
            }
            StepPhase::Executing => match runner.poll(&step) {
                Err(error) => {
                    let reason = format!("skill '{}' poll failed: {error}", step.skill);
                    self.finish_aborted(active, &step, reason, runner)
                }
                Ok(SkillPoll::Running) => {
                    let status = ExecutorStatus::Executing { step_id: step.id };
                    self.active = Some(active);
                    Ok(status)
                }
                Ok(SkillPoll::Succeeded) => {
                    self.events.record(
                        EventKind::SkillCompleted,
                        Some(&step.skill),
                        format!("step '{}' completed", step.id),
                    );
                    active.step_index += 1;
                    if active.step_index == active.plan.steps.len() {
                        let status = ExecutorStatus::Completed {
                            plan_id: active.plan.id.clone(),
                        };
                        self.events.record(
                            EventKind::PlanCompleted,
                            Some(&active.plan.id),
                            "all plan steps completed",
                        );
                        self.terminal = Some(status.clone());
                        Ok(status)
                    } else {
                        active.step_activated_ms = now_ms;
                        active.phase = StepPhase::Ready;
                        active.approved_gate = None;
                        let status = ExecutorStatus::StepCompleted { step_id: step.id };
                        self.active = Some(active);
                        Ok(status)
                    }
                }
                Ok(SkillPoll::Failed(detail)) => {
                    let reason = format!("skill '{}' failed: {detail}", step.skill);
                    self.finish_aborted(active, &step, reason, runner)
                }
            },
        }
    }

    pub fn approve_gate(&mut self, principal: &AuthContext, gate_id: &str) -> eyre::Result<()> {
        if !principal.is_authenticated() || principal.role() == Role::Observer {
            return Err(eyre::eyre!(
                "human gate approval requires an authenticated operator"
            ));
        }
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| eyre::eyre!("no active plan"))?;
        if active.phase != StepPhase::AwaitingGate {
            return Err(eyre::eyre!("executor is not awaiting a human gate"));
        }
        let step = &active.plan.steps[active.step_index];
        let gate = step
            .human_gate
            .as_ref()
            .ok_or_else(|| eyre::eyre!("current step has no human gate"))?;
        if gate.id != gate_id {
            return Err(eyre::eyre!("approval does not match the active gate"));
        }
        active.approved_gate = Some(gate.id.clone());
        active.phase = StepPhase::Ready;
        self.events.record(
            EventKind::HumanGateApproved,
            Some(principal.subject()),
            format!("gate '{}' approved for step '{}'", gate.id, step.id),
        );
        Ok(())
    }

    pub fn abort_active<R: SkillRunner>(
        &mut self,
        reason: &str,
        runner: &mut R,
    ) -> eyre::Result<ExecutorStatus> {
        if reason.trim().is_empty() {
            return Err(eyre::eyre!("abort reason must not be empty"));
        }
        let active = self
            .active
            .take()
            .ok_or_else(|| eyre::eyre!("no active plan"))?;
        let step = active.plan.steps[active.step_index].clone();
        self.finish_aborted(active, &step, reason.to_owned(), runner)
    }

    fn finish_aborted<R: SkillRunner>(
        &mut self,
        active: ActivePlan,
        step: &PlanStep,
        mut reason: String,
        runner: &mut R,
    ) -> eyre::Result<ExecutorStatus> {
        if active.phase == StepPhase::Executing {
            if let Err(error) = runner.abort(step, &reason) {
                reason.push_str(&format!("; runner abort failed: {error}"));
            }
        }
        self.events.record(
            EventKind::SkillFailed,
            Some(&step.skill),
            format!("step '{}' stopped: {reason}", step.id),
        );
        self.events.record(
            EventKind::PlanAborted,
            Some(&active.plan.id),
            reason.clone(),
        );
        let status = ExecutorStatus::Aborted {
            plan_id: active.plan.id,
            reason,
        };
        self.terminal = Some(status.clone());
        Ok(status)
    }

    fn validate_skill_spec(&self, spec: &SkillSpec) -> eyre::Result<()> {
        if spec.name.trim().is_empty() {
            return Err(eyre::eyre!("skill name must not be empty"));
        }
        if spec
            .required_preconditions
            .iter()
            .chain(spec.required_abort_conditions.iter())
            .any(|condition| condition.key.trim().is_empty())
        {
            return Err(eyre::eyre!("skill condition names must not be empty"));
        }
        Ok(())
    }

    fn validate_conditions(&self, step: &PlanStep, spec: &SkillSpec) -> eyre::Result<()> {
        if step
            .preconditions
            .iter()
            .chain(step.abort_conditions.iter())
            .any(|condition| condition.key.trim().is_empty())
        {
            return Err(eyre::eyre!("step '{}' has an empty condition", step.id));
        }
        for required in &spec.required_preconditions {
            if !step.preconditions.contains(required) {
                return Err(eyre::eyre!(
                    "step '{}' omits required precondition '{}={}'",
                    step.id,
                    required.key,
                    required.expected
                ));
            }
        }
        for required in &spec.required_abort_conditions {
            if !step.abort_conditions.contains(required) {
                return Err(eyre::eyre!(
                    "step '{}' omits required abort condition '{}={}'",
                    step.id,
                    required.key,
                    required.expected
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Condition, HumanGate};
    use crate::skills::{actuate_handle_spec, inspect_fixture_spec};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct MockRunner {
        starts: Vec<String>,
        aborts: Vec<String>,
        polls: VecDeque<SkillPoll>,
        poll_error: bool,
    }

    impl SkillRunner for MockRunner {
        fn start(&mut self, step: &PlanStep) -> eyre::Result<()> {
            self.starts.push(step.id.clone());
            Ok(())
        }

        fn poll(&mut self, _step: &PlanStep) -> eyre::Result<SkillPoll> {
            if self.poll_error {
                return Err(eyre::eyre!("runner unavailable"));
            }
            Ok(self.polls.pop_front().unwrap_or(SkillPoll::Running))
        }

        fn abort(&mut self, step: &PlanStep, _reason: &str) -> eyre::Result<()> {
            self.aborts.push(step.id.clone());
            Ok(())
        }
    }

    fn condition(key: &str, expected: bool) -> Condition {
        Condition {
            key: key.to_owned(),
            expected,
        }
    }

    fn gated_spec() -> SkillSpec {
        SkillSpec {
            name: "gated_skill".to_owned(),
            max_level: AutonomyLevel::L3,
            required_preconditions: vec![condition("ready", true)],
            required_abort_conditions: vec![condition("fault", true)],
        }
    }

    fn step(id: &str, level: AutonomyLevel, gate: bool) -> PlanStep {
        PlanStep {
            id: id.to_owned(),
            skill: "gated_skill".to_owned(),
            requested_level: level,
            timeout_ms: 100,
            preconditions: vec![condition("ready", true)],
            abort_conditions: vec![condition("fault", true)],
            human_gate: gate.then(|| HumanGate {
                id: format!("gate-{id}"),
                prompt: "approve execution".to_owned(),
            }),
        }
    }

    fn executor() -> eyre::Result<(PlanExecutor, EventLog)> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 128)?;
        let mut executor = PlanExecutor::new(clock, events.clone(), 8, 4)?;
        executor.register_skill(gated_spec())?;
        Ok((executor, events))
    }

    #[test]
    fn preconditions_and_autonomy_level_are_validated() -> eyre::Result<()> {
        let (executor, _) = executor()?;
        let mut invalid = step("one", AutonomyLevel::L4, false);
        let plan = TaskPlan {
            id: "plan".to_owned(),
            steps: vec![invalid.clone()],
        };
        assert!(executor.validate_plan(&plan).is_err());
        invalid.requested_level = AutonomyLevel::L2;
        invalid.preconditions = vec![condition("ready", false)];
        let plan = TaskPlan {
            id: "plan".to_owned(),
            steps: vec![invalid],
        };
        assert!(executor.validate_plan(&plan).is_err());
        Ok(())
    }

    #[test]
    fn runner_poll_error_aborts_and_records_plan() -> eyre::Result<()> {
        let (mut executor, events) = executor()?;
        executor.start_plan_at(
            TaskPlan {
                id: "runner-error".to_owned(),
                steps: vec![step("one", AutonomyLevel::L2, false)],
            },
            0,
        )?;
        let facts = FactSet::from([("ready".to_owned(), true), ("fault".to_owned(), false)]);
        let mut runner = MockRunner {
            poll_error: true,
            ..MockRunner::default()
        };
        executor.tick_at(1, &facts, &mut runner)?;
        assert!(matches!(
            executor.tick_at(2, &facts, &mut runner)?,
            ExecutorStatus::Aborted { .. }
        ));
        assert_eq!(runner.aborts, ["one"]);
        assert!(events.to_jsonl().contains("runner unavailable"));
        Ok(())
    }

    #[test]
    fn checked_in_w1_plan_matches_registered_skill_envelopes() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 64)?;
        let mut executor = PlanExecutor::new(clock, events, 8, 4)?;
        executor.register_skill(inspect_fixture_spec())?;
        executor.register_skill(actuate_handle_spec())?;
        let plan = TaskPlan::from_json(include_str!("../../../plans/w1_inspect_and_handle.json"))?;
        executor.validate_plan(&plan)
    }

    #[test]
    fn multi_step_plan_waits_for_gate_then_completes() -> eyre::Result<()> {
        let (mut executor, events) = executor()?;
        let plan = TaskPlan {
            id: "two-step".to_owned(),
            steps: vec![
                step("one", AutonomyLevel::L3, true),
                step("two", AutonomyLevel::L2, false),
            ],
        };
        executor.start_plan_at(plan, 0)?;
        let facts = FactSet::from([("ready".to_owned(), true), ("fault".to_owned(), false)]);
        let mut runner = MockRunner {
            polls: VecDeque::from([SkillPoll::Succeeded, SkillPoll::Succeeded]),
            ..MockRunner::default()
        };
        assert!(matches!(
            executor.tick_at(1, &facts, &mut runner)?,
            ExecutorStatus::AwaitingGate { .. }
        ));
        let observer = AuthContext::authenticated("observer", Role::Observer)?;
        assert!(executor.approve_gate(&observer, "gate-one").is_err());
        let operator = AuthContext::authenticated("operator", Role::Operator)?;
        executor.approve_gate(&operator, "gate-one")?;
        assert!(matches!(
            executor.tick_at(2, &facts, &mut runner)?,
            ExecutorStatus::Executing { .. }
        ));
        assert!(matches!(
            executor.tick_at(3, &facts, &mut runner)?,
            ExecutorStatus::StepCompleted { .. }
        ));
        assert!(matches!(
            executor.tick_at(4, &facts, &mut runner)?,
            ExecutorStatus::Executing { .. }
        ));
        assert!(matches!(
            executor.tick_at(5, &facts, &mut runner)?,
            ExecutorStatus::Completed { .. }
        ));
        assert_eq!(runner.starts, ["one", "two"]);
        let log = events.to_jsonl();
        assert!(log.contains("HumanGateRequested"));
        assert!(log.contains("HumanGateApproved"));
        assert!(log.contains("PlanCompleted"));
        Ok(())
    }

    #[test]
    fn failed_precondition_blocks_skill_start_until_timeout() -> eyre::Result<()> {
        let (mut executor, events) = executor()?;
        executor.start_plan_at(
            TaskPlan {
                id: "blocked".to_owned(),
                steps: vec![step("one", AutonomyLevel::L2, false)],
            },
            0,
        )?;
        let facts = FactSet::from([("ready".to_owned(), false), ("fault".to_owned(), false)]);
        let mut runner = MockRunner::default();
        assert!(matches!(
            executor.tick_at(50, &facts, &mut runner)?,
            ExecutorStatus::WaitingPreconditions { .. }
        ));
        assert!(runner.starts.is_empty());
        assert!(matches!(
            executor.tick_at(101, &facts, &mut runner)?,
            ExecutorStatus::Aborted { .. }
        ));
        assert!(events.to_jsonl().contains("PlanAborted"));
        Ok(())
    }

    #[test]
    fn abort_condition_stops_running_skill() -> eyre::Result<()> {
        let (mut executor, _) = executor()?;
        executor.start_plan_at(
            TaskPlan {
                id: "faulted".to_owned(),
                steps: vec![step("one", AutonomyLevel::L2, false)],
            },
            0,
        )?;
        let nominal = FactSet::from([("ready".to_owned(), true), ("fault".to_owned(), false)]);
        let faulted = FactSet::from([("ready".to_owned(), true), ("fault".to_owned(), true)]);
        let mut runner = MockRunner::default();
        executor.tick_at(1, &nominal, &mut runner)?;
        assert!(matches!(
            executor.tick_at(2, &faulted, &mut runner)?,
            ExecutorStatus::Aborted { .. }
        ));
        assert_eq!(runner.aborts, ["one"]);
        Ok(())
    }

    #[test]
    fn executor_capacities_are_enforced() -> eyre::Result<()> {
        let clock = RuntimeClock::new();
        let events = EventLog::new(clock, 32)?;
        let mut executor = PlanExecutor::new(clock, events, 1, 1)?;
        executor.register_skill(gated_spec())?;
        assert!(executor.register_skill(gated_spec()).is_err());
        let plan = TaskPlan {
            id: "oversized".to_owned(),
            steps: vec![
                step("one", AutonomyLevel::L2, false),
                step("two", AutonomyLevel::L2, false),
            ],
        };
        assert!(executor.validate_plan(&plan).is_err());
        Ok(())
    }
}
