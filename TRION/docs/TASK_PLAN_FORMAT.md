# Task Plan Format — v0.1

Trion task plans are version-controlled JSON artifacts consumed by `trion-skills`. The bounded executor validates the entire plan before execution, then evaluates live preconditions, abort conditions, timeouts, autonomy ceilings, and human go/no-go gates at runtime.

The first reference artifact is [`w1_inspect_and_handle.json`](../plans/w1_inspect_and_handle.json).

## Schema

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | non-empty string | Stable plan identifier used in event records |
| `steps` | array | Ordered steps; length is bounded by executor configuration |
| `steps[].id` | unique non-empty string | Step identifier |
| `steps[].skill` | string | Name of a registered `SkillSpec` |
| `steps[].requested_level` | `L0`…`L4` | Requested autonomy level; must not exceed the skill's recorded maximum |
| `steps[].timeout_ms` | positive integer | Maximum time from step activation, including precondition and gate waits |
| `steps[].preconditions` | condition array | Facts that must match before the skill starts |
| `steps[].abort_conditions` | condition array | Facts that abort an active or waiting step when matched |
| `steps[].human_gate` | object or `null` | Optional approval gate with non-empty `id` and operator prompt; mandatory at L3 |

A condition has a string `key` and Boolean `expected` value. It is satisfied only when the current fact set contains that exact key/value pair. A missing precondition blocks start. A missing abort fact does not abort.

## Validation and execution

Before accepting a plan, `PlanExecutor` rejects empty or oversized plans, duplicate step IDs, zero timeouts, unknown skills, autonomy requests above the registered ceiling, malformed gates, and steps that omit any safety condition required by their skill specification.

Execution follows this sequence:

1. Evaluate abort conditions and the step deadline.
2. Wait until all preconditions match.
3. If configured, emit a gate request and wait for an authenticated Operator or Supervisor approval for the exact gate ID.
4. Start and poll the skill runner until success, failure, abort, or timeout.
5. Advance to the next step or emit a terminal plan event.

The event log records plan start/completion/abort, gate request/approval, and skill start/completion/failure. Approval authenticates the human decision; actuator commands produced by a skill must still pass through `trion-runtime` command authority and the safety shim.

## Current boundary

This v0.1 format and deterministic executor are implemented and unit tested. The canonical skill entries currently define execution envelopes only: `inspect_fixture` and `actuate_handle` are capped at L2, while `mate_connector` is capped at L1. Motion implementations, perception facts, durable mission-control upload, commit-window coupling for contact actions, MuJoCo execution, and promotion evidence remain Phase 2 integration work.
