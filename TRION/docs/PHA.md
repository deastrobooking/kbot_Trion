# Preliminary Hazard Analysis (PHA) — Trion Space Operations Engineer

**System:** Trion robot (K-Bot + Trion runtime/skills/perception) and its ground segment
**Phase analyzed:** Analog ground operations (habitats, ground stations, mock worksites)
**Version:** v1 (2026-08-02) — expanded with system definition, RAC methodology, and four new hazards after external review
**Companion:** the safety case built on this analysis lives in [SAFETY_CASE.md](SAFETY_CASE.md)

Modeled on NASA/ISS practice (Robonaut 2 lessons learned): safety is priority one, safety systems are verified meticulously, and safety logic is separated from control logic and kept local to the robot. This document feeds [FDIR_MATRIX.md](FDIR_MATRIX.md), [REQUIREMENTS.md](REQUIREMENTS.md), and [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md).

## 1. System definition & boundaries

**Mission phases considered:**
- Teleoperated inspection and maintenance (L0–L1)
- Supervised autonomous task execution (L2–L3): connectors, handles, fasteners, ORU-style modules
- Fault recovery and safe-mode entry/exit

**Included in the analysis:**
- K-Bot hardware: actuators (48 V CAN bus), IMU, cameras, compute, power board
- Trion runtime (Rust): FDIR supervisor, health bus, safe-mode logic, command auth
- Trion skills (manipulation, locomotion, inspection) and perception (localization, fixture recognition)
- Ground segment: mission-control console, gRPC link

**Excluded (explicit non-goals at this phase):**
- Actual space environment (vacuum, radiation, thermal extremes)
- Facility infrastructure beyond the mock worksites

## 2. Method

Hazards are identified from energy sources (electrical, mechanical, thermal), hazardous functions (motion, contact, tool use), hazardous components (actuators, batteries, compute), and lessons learned from Robonaut 2, Dextre, Astrobee, and GITAI.

Risk assessment:
- **Severity:** Catastrophic (death/severe injury or facility loss) · Critical (injury or major robot/worksite damage) · Marginal (minor damage or mission degradation) · Negligible
- **Probability (pre-mitigation, engineering judgment at v1):** Frequent · Probable · Occasional · Remote · Improbable
- **RAC (Risk Assessment Code):** High / Medium / Low from the severity × probability combination

## 3. Hazard worksheet

| ID | Phase | Hazard | Effect | Sev | Prob | RAC | Controls (→ requirements / FDIR) | Verified by |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| H-01 | All | Uncontrolled motion near humans or equipment | Impact, crush, equipment damage | Catastrophic | Occasional | High | Hard joint/velocity/torque limits enforced below the policy layer; hardware + software e-stop (TR-P4-042a); safe-mode within 100 ms (TR-P4-020); watchdog + FDIR (TR-P4-010…013, F-01/F-05/F-06/F-09); safety/control separation (TR-P4-022) | Timed safe-mode tests; fault-injection campaign |
| H-02 | Manipulation | Excessive contact force at worksite (crushed connector, bent pins, damaged panel) | Worksite damage, mission failure | Critical | Probable | High | Per-skill force/torque limits; commit-window approval before contact (TR-P4-042c); Dextre-style force-moment monitoring with abort thresholds | Skill force-limit tests (sim + hardware) |
| H-03 | Manipulation | Dropped tool or module | Damage, FOD, mission failure | Critical | Occasional | Medium | Grasp verification (force + vision) before transport; retention checklist in task plans; drop-detection event | Grasp-failure injection scenarios |
| H-04 | Locomotion | Fall / tip-over (legged operation) | Robot/worksite damage | Critical | Occasional | Medium | Mounted-torso mode for manipulation-heavy tasks (Apollo pattern); disturbance-trained locomotion policies; fall detection → zero-torque crouch | Locomotion campaign metrics; fall-response test |
| H-05 | Servicing | Electrical hazard (48 V bus, CAN wiring) | Shock, short, fire | Catastrophic | Remote | Medium | Worksite electrical review; insulated end-effector surfaces; power-off preconditions encoded in task plans | Procedure review; inspection checklist |
| H-06 | All | Actuator overcurrent / thermal runaway | Fire, component damage, loss of control | Critical | Occasional | High | Per-joint current/temperature monitoring with derating and isolation (F-02, F-03); duty-cycle limits in skills; safe posture on critical threshold | Fault injection (F-02/F-03); hardware soak test |
| H-07 | Shared control / autonomy | Pinch points during motion near people | Injury | Catastrophic | Occasional | High | Keep-out zone during L2+ operation; speed reduction near humans; predictable motion (no un-commanded reposturing) | Ops procedure; human-factors review |
| H-08 | Operations | Unauthorized, spoofed, or **incorrect** command execution (wrong task/step) | Damage, mission failure | Critical | Remote | Medium | Command authentication + role-based classes + two-step arming (TR-P4-050); precondition checks in the task executor; full command audit log (TR-P4-031) | Security review; precondition-failure tests |
| H-09 | Power | Battery overcharge / over-discharge / BMS fault | Fire, thermal event, loss of power mid-task | Catastrophic | Remote | Medium | BMS hardware limits (first line, independent of software); runtime monitors SoC/temperature; F-08 load-shed → controlled shutdown posture | BMS review; fault injection (F-08) |
| H-10 | Teleop / autonomy | Comms loss during a contact-rich task | Robot stuck in contact, progressive damage | Critical | Occasional | High | Command-heartbeat timeout (TR-P4-043, F-07); on loss: halt motion, hold or retract per skill policy; safe-mode if contact persists beyond threshold | Comms-drop tests during contact (sim + hardware) |
| H-11 | Inspection | False-negative anomaly detection (damage missed) | Latent failure discovered later | Marginal | Occasional | Medium | Redundant checks (visual + geometric); human review of inspection results in early phases; versioned inspection policies with regression tests | Inspection regression suite |
| H-12 | All | Software defect in FDIR or safe-mode logic itself | Failure to respond correctly to a real fault | Catastrophic | Remote | Medium | Sim-first V&V with fault injection; requirements-to-test traceability ([REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md)); coding standard TR-P4-070 (review, clippy, bounded loops/memory); staged rollout of FDIR changes | 100% FDIR entries exercised per release |

## 4. Risk ranking & follow-up actions

**High-RAC hazards (H-01, H-02, H-06, H-07, H-10) must have, before any Phase 1+ hardware operation:**
- Explicit requirements in [REQUIREMENTS.md](REQUIREMENTS.md)
- FDIR entries in [FDIR_MATRIX.md](FDIR_MATRIX.md)
- Sim tests (and hardware tests where applicable) in [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md)

**Medium-RAC hazards** are addressed in Phases 1–2 with procedural and software controls, and reviewed at every autonomy-level promotion ([AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md)).

## 5. Conclusions

- The Trion architecture (local safety logic, FDIR, safe-mode, command authority) aligns with the mitigations for every high-RAC hazard.
- Priority focus areas: contact-rich manipulation safety (H-01, H-02), FDIR/watchdog behavior under faults (H-06, H-12), and comms-loss behavior during contact (H-10).
- This PHA is a living document: update when adding subsystems (tool changers, new sensors), worksites, or tasks, and after any incident or near-miss.

**Working rule (Robonaut 2):** no capability is promoted up the autonomy ladder until the safety goals it touches have evidence at the current level. Safety first, capability second.
