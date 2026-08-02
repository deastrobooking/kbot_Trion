# Requirements-to-Test Traceability Matrix — v0.1

**Purpose:** every safety-critical and mission-critical requirement is verified by one or more tests (sim and/or hardware).
**Scope:** all TR-P4 requirements ([REQUIREMENTS.md](REQUIREMENTS.md)); safety-related P1–P3 requirements join as those pillars gain requirements.
**Version:** v0.1 (2026-08-02) — updated with planned tests from [RECOMMENDATIONS_AND_REWRITES.md](RECOMMENDATIONS_AND_REWRITES.md).

Test ID conventions: `RT-*` = Rust test implemented in the Trion Rust workspace · `T-SIM-*` = simulation scenario · `T-HW-*` = hardware test. Status: ✅ passing · 🔜 planned.

## Matrix

| Req ID | Test ID | Test | Type | Pass criteria | Status |
| --- | --- | --- | --- | --- | --- |
| TR-P4-001 | RT-01 | `telemetry::health::tests::publish_reaches_subscriber_and_counts_drops` | Unit | Report reaches subscriber with service/level/timestamp | ✅ |
| TR-P4-002 | RT-01 | (same — monotonic timestamps) | Unit | Timestamps from single monotonic base | ✅ (robot side) |
| TR-P4-002 | T-HW-04 | Ground↔robot clock-skew measurement | Hardware | Skew ≤ 50 ms over a session | 🔜 |
| TR-P4-003 | RT-01, RT-02 | Subscriber-overflow accounting + `telemetry::events::tests::log_is_bounded_and_exports_jsonl` | Unit | No unbounded growth; unobserved and lag-evicted reports counted; oldest event evicted | ✅ |
| TR-P4-010 | RT-03 | `runtime::healthy_service_produces_no_fault` | Integration | Beating service never trips watchdog | ✅ |
| TR-P4-011 | RT-04 | `runtime::missed_heartbeat_detected_within_bound` | Integration | Deterministic paused-time detection at, and not before, (miss_limit + 1) × interval; slow monitor ticks are clamped | ✅ |
| TR-P4-012 | RT-05 | `runtime::repeated_faults_escalate_to_safe_mode_and_recovery_is_explicit` | Integration | Restart budget exhausted → safe-mode | ✅ |
| TR-P4-013 | T-SIM-03 | Per-release FDIR sweep (every F-xx entry injected once, e.g. IMU dropout mid-task) | Sim | Each entry: detect → isolate → recover/escalate as specified | 🔜 |
| TR-P4-020 | RT-05, RT-10 | Safe-mode state entry + supervisor-to-actuator transport path | Integration | Mode latches without subscribers; 20-actuator safe state commanded within 100 ms and event recorded | ✅ (simulated transport) / 🔜 (MuJoCo + hardware) |
| TR-P4-020 | T-SIM-01 / T-HW-01 | Timed safe-mode on injected actuator overcurrent, incl. zero-torque path | Sim + HW | Entry ≤ 100 ms of decision; no joint exceeds limits | 🔜 |
| TR-P4-021 | RT-05 | Explicit-recovery-only verified | Integration | No auto-exit; audited operator exit works | ✅ |
| TR-P4-022 | — | Dependency audit: `trion-runtime` has no perception/network deps | Review/CI | `cargo tree` contains no such crates; architecture review at phase gates | ✅ (manual) / 🔜 (CI check) |
| TR-P4-030 | RT-02, RT-11 | Bounded event log + plan/gate/skill lifecycle events | Unit | Capacity enforced; valid JSONL; supervised plan lifecycle is recorded | ✅ |
| TR-P4-031 | T-SIM-07 | Tamper-evident log verification | Sim/ground | Hash chain detects modification | 🔜 |
| TR-P4-040 | T-SIM-08 | Latency-injection teleop test (200 ms and 2 s) | Sim | Task-level operation correct at 2 s one-way | 🔜 |
| TR-P4-041 | T-SIM-09 | Degraded comms modes | Sim | Telemetry-only + store-and-forward transitions logged | 🔜 |
| TR-P4-042 | T-SIM-10 | Command classes: immediate / queued / commit-window | Sim | E-stop always accepted; queued preconditions enforced; expired commit windows rejected | 🔜 |
| TR-P4-043 | T-SIM-05 | Comms loss during contact (H-10) | Sim | Halt within bound; hold to gate; safe hold after timeout; no unsafe motion | 🔜 |
| TR-P4-050 | T-SIM-11 | Command auth: role classes, two-step arming, replay rejection | Sim/ground | Unauthorized + unarmed hazardous commands rejected and logged | 🔜 |
| TR-P4-060 | T-HW-05 | WCET measurement of control path + frame-overrun policy | Hardware | Measured WCET documented; N overruns → safe-mode | 🔜 |
| TR-P4-070 | CI | clippy + fmt + test + demo gate | CI | Zero violations on PR | ✅ |
| TR-P4-042 | RT-06 | `command::authority` class and commit-window tests | Unit | Authenticated immediate safety path remains available; queued role checks enforced; unarmed, expired, mismatched, or consumed approvals rejected and logged | ✅ |
| TR-P4-042 | RT-11 | `trion-skills::executor` precondition, timeout, gate, abort, capacity, and multi-step tests | Unit | Invalid plans are rejected; failed preconditions block start; timeouts and abort facts stop execution; exact authenticated gate approval is required; plan and registries remain bounded | ✅ (executor core) / 🔜 (MuJoCo coupling) |
| F-09 | RT-07 | `command::gate` clips out-of-limit actuator command and rejects commands in safe-mode | Unit | Position/velocity/torque clamped to manifest limits; unknown/non-finite commands rejected; safe-mode blocks ordinary commands | ✅ (gate) / 🔜 (violation escalation) |
| TR-P4-050 | RT-06 | Role, anti-replay, capacity, and two-step approval tests | Unit | Unauthenticated and underprivileged commands rejected; queued/commit replay rejected; authority state bounded | ✅ (authority) / 🔜 (credential adapter + security review) |
| F-05 (IMU stale) | RT-08 | `fdir::imu_monitor` detects stale IMU and emits event | Unit | Missing samples for > threshold → FaultDetected + health Critical | 🔜 |
| P1 (E-stop) | T-SIM-13 | E-stop stops motion within 100 ms in sim | Sim | Safe-mode entered; zero-torque issued within bound | 🔜 |
| P1 (E-stop) | T-HW-03 | E-stop during motion | Hardware | Immediate halt regardless of command source; no rebound | 🔜 |
| P2 (policy service) | T-SIM-12 | `trion-policy-service` runs worksite W1 dry-run | Sim | ONNX inference loop executes without direct hardware deps | 🔜 |
| P2 (robot manifest) | RT-09 | Robot manifest validation rejects duplicate IDs and invalid limits | Unit | Startup validation fails with descriptive error; unreviewed manifest refuses hardware approval | ✅ |
| Phase 0 digital twin | T-SIM-14 | `compose_w1_kbot.py` loads pinned K-Bot + W1 | Sim | Named robot, fixture targets, 20 actuators, and finite state after 100 steps | ✅ |
| P3 (skill preconditions) | RT-11 | Skill invoked with failed precondition (e.g., poor localization) | Unit | Skill runner is not started; wait state identifies missing facts; deadline abort is recorded | ✅ |
| P3 (skill preconditions) | T-SIM-06 | Skill invoked with failed precondition (e.g., poor localization) | Sim | Skill refuses to start; operator notified | 🔜 |

## Coverage targets

- **Safety-critical requirements:** 100% mapped to ≥ 1 sim test, plus a hardware test where applicable — before Phase 2 hardware operations.
- **FDIR entries:** 100% exercised in sim per release (TR-P4-013 sweep).
- **Flagship skills:** promotion campaigns per [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md) (≥ 50 sim / ≥ 10 hardware runs).

## Maintenance

Updated when requirements or tests are added and before each release candidate. CI will enforce: every requirement row marked safety-critical in [REQUIREMENTS.md](REQUIREMENTS.md) must have at least one non-planned test, or the release check fails.
