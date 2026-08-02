# Trion P4 Requirements — v0

Numbered requirements for the mission-critical runtime pillar (P4 in [PROJECT_PLAN.md](PROJECT_PLAN.md)). Every requirement carries a verification method; [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md) links each to its verifying tests. Hazard links refer to [PHA.md](PHA.md); fault links refer to [FDIR_MATRIX.md](FDIR_MATRIX.md); the safety argument lives in [SAFETY_CASE.md](SAFETY_CASE.md).

Status legend: ✅ implemented in `trion-runtime` v0 skeleton · 🔜 planned.

## Health telemetry

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-001 | Every runtime-supervised service SHALL publish a health report (service, level ∈ {Nominal, Degraded, Critical}, message, timestamp) at ≥ 1 Hz. | Unit test + demo | ✅ |
| TR-P4-002 | All health reports, events, and logs SHALL be stamped from a single monotonic time base per robot; ground↔robot skew SHALL be bounded and documented (target ≤ 50 ms via NTP; robot uses monotonic clock, mapped to wall time at session start). | Analysis + test | ✅ (monotonic base) / 🔜 (ground sync) |
| TR-P4-003 | The telemetry bus SHALL use bounded memory (fixed channel capacity); overflow SHALL drop oldest data and increment a counted, reported statistic — never block the publisher or grow without bound. | Unit test | ✅ |

## Watchdog & FDIR

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-010 | Every supervised service SHALL register a heartbeat with a per-service interval and miss limit. | Unit test | ✅ |
| TR-P4-011 | A missed-heartbeat fault SHALL be detected within (miss_limit + 1) × interval of the last heartbeat. Links: F-01. | Unit test (bounded-time assertion) | ✅ |
| TR-P4-012 | On a service-down fault the supervisor SHALL attempt restart; if restarts exceed max_restarts within the escalation window, the runtime SHALL enter safe-mode. Links: F-01, H-01. | Unit test + demo | ✅ |
| TR-P4-013 | Every fault class in [FDIR_MATRIX.md](FDIR_MATRIX.md) SHALL have a detection signal, isolation rule, recovery action, and safe-mode trigger condition, each exercised at least once in sim per release. | Sim fault-injection campaign | 🔜 |

## Safe-mode

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-020 | Safe-mode entry SHALL complete within 100 ms of the triggering decision: commanded zero-torque/brake posture, heartbeat telemetry continues, all non-recovery commands rejected. Links: H-01, H-02. | Timed test (sim + hardware) | ✅ (state machine) / 🔜 (actuator path) |
| TR-P4-021 | Safe-mode SHALL exit only on an explicit, authenticated recovery command; never automatically. | Unit test | ✅ |
| TR-P4-022 | **Safety/control separation (design constraint):** safety logic — limits, e-stop, safe-mode, watchdogs — SHALL NOT depend on non-critical services (high-level perception, network links, mission-control connectivity) and SHALL run local to the robot. | Architecture review at each phase gate | ✅ (crate has no perception/network deps) |

## Event logging

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-030 | The runtime SHALL keep a structured event log (mode changes, fault detections, recovery attempts/results, safe-mode entry/exit, human gates, skill start/stop) with bounded memory and JSONL export; events are the source for post-mission reports. | Unit test + report generator | ✅ (bounded log + plan/gate/skill events) / 🔜 (reports) |
| TR-P4-031 | Mission logs SHALL be tamper-evident (append-only storage with integrity hashes) in the ground segment. | Design review + test | 🔜 |

## Comms & command model

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-040 | Latency budgets: teleop video+state ≤ 200 ms one-way on the local analog link; all task-level operation SHALL remain correct at up to 2 s one-way latency (design-for-delay). | Latency injection test | 🔜 |
| TR-P4-041 | Degraded comms modes SHALL exist: low-bandwidth telemetry-only downlink, and store-and-forward command uplink; mode transitions are logged events. | Sim + link-shaping test | 🔜 |
| TR-P4-042 | Commands SHALL be classed: (a) **immediate** (e-stop, safe-mode — always accepted, minimal path), (b) **queued** task steps with preconditions and timeouts, (c) **commit-window** contact actions (operator approves; robot must begin execution within a bounded window or the approval expires). | Unit + integration tests | ✅ authority + queued executor core / 🔜 end-to-end sim coupling |
| TR-P4-043 | Loss of ground comms SHALL NOT cause unsafe behavior: the robot completes the current queued step to its next gate, then holds; after a configured timeout it enters safe hold. Links: F-07. | Sim fault injection | 🔜 |

## Command authority & security

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-050 | All commands SHALL be authenticated; command classes are role-based (observer / operator / supervisor); hazardous commands (motion in contact, power cycling, safe-mode exit) require two-step arming. Links: H-08. | Security review + tests | ✅ bounded authority core / 🔜 credential adapter + security review |

## Control loop determinism

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-060 | The control/inference path SHALL have a measured worst-case execution time; a frame-overrun policy SHALL be defined (skip-frame + degrade rate; N consecutive overruns → safe-mode). Links: F-06. | WCET measurement campaign | 🔜 |

## Coding standard

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| TR-P4-070 | Trion Rust code SHALL follow: no `unwrap()`/`expect()` outside tests-of-invariants, `eyre` for errors, `tracing` for logs, bounded loops and bounded post-init memory, every `Result` checked, invariant assertions. Enforced by clippy config + review. | CI lint + review | ✅ (applied in trion-runtime) |
