# FDIR Matrix — v0

Fault Detection, Isolation, and Recovery for the Trion runtime. Each entry must be exercised at least once in sim per release (TR-P4-013). Hazard links refer to [PHA.md](PHA.md).

Status: ✅ implemented in `trion-runtime` v0 skeleton · 🔜 planned.

| ID | Fault | Detection signal | Isolation | Recovery | Safe-mode trigger | Hazard | Status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F-01 | Service crash / hang | Missed heartbeats > miss_limit (watchdog) | Which service, via heartbeat registry | Supervisor restarts service | Restarts exceed max_restarts within window | H-01 | ✅ |
| F-02 | Actuator overtemperature | Temp telemetry > derate threshold; > critical threshold | Joint ID from CAN feedback | Torque derating on that joint; pause task step | Critical threshold, or derating fails to arrest rise | H-06 | 🔜 |
| F-03 | Actuator overcurrent | Current sense > limit for > N ms | Joint ID | Cut torque on joint, retry step once at reduced effort | Second occurrence in same task step | H-01, H-06 | 🔜 |
| F-04 | CAN bus communication loss | Feedback frame timeout | Single actuator (one ID silent) vs bus (all silent) | Single: mark joint degraded, hold. Bus: one bus reset attempt | Bus reset fails, or degraded joint is load-bearing | H-01 | 🔜 |
| F-05 | IMU dropout / stale data | Timestamp staleness > 2× sample period | Which IMU (hexmove/hiwonder) | Switch to redundant IMU if present; else freeze locomotion, hold posture | Stale during locomotion or contact task | H-01, H-04 | 🔜 |
| F-06 | Control-loop frame overrun | Loop time > budget (WCET monitor) | Control path stage timing | Skip-frame policy; degrade control rate | N consecutive overruns | H-01 | 🔜 |
| F-07 | Ground comms loss | Link heartbeat timeout | Network layer vs mission-control process | Complete current queued step to next gate, then hold | Hold exceeds configured timeout | H-01 | 🔜 |
| F-08 | Power board fault / undervoltage | `kbot-pwrbrd` telemetry out of range | Rail / board channel | Load-shed non-critical services | Voltage below safe-operation floor → controlled shutdown posture | H-01 | 🔜 |
| F-09 | Joint limit / self-collision imminent | Kinematic monitor on commanded trajectory | Offending command + joint | Clamp command, stop current motion segment | Repeated violations in one task (indicates bad state estimate) | H-01, H-02 | 🔜 |
| F-10 | State-estimate divergence (localization loss) | Filter innovation/covariance beyond threshold | Perception tier (non-safety-critical) | Stop motion, hold, attempt re-localization | Re-localization fails within timeout during a contact task | H-01, H-02 | 🔜 |

## Conventions

- **Detect → isolate → recover → escalate** is the invariant pipeline: every fault gets one bounded recovery attempt path before safe-mode escalation; no infinite retry loops (TR-P4-070).
- Every detection, recovery attempt, and escalation is a structured event in the event log (TR-P4-030) — the fault record is the evidence for the safety case.
- Safe-mode itself is specified in TR-P4-020/021: zero-torque/brake posture, telemetry heartbeat continues, only authenticated recovery commands accepted.
