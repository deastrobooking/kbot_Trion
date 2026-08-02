# Safety Case — Trion Space Operations Engineer (v0)

A structured argument, with evidence, that the system is acceptably safe for its intended use. Built on [PHA.md](PHA.md); becomes Safety Case v1 in Phase 4 with accumulated campaign evidence.

**Intended use:** analog ground operations in human-rated facilities (habitats, ground stations, mock worksites).
**Not intended for:** actual spaceflight, high-radiation, vacuum, or extreme thermal environments.
**Version:** v0 (2026-08-02)

## 1. Safety goals

| ID | Goal |
| --- | --- |
| SG-1 | The robot shall not cause catastrophic injury to humans or catastrophic damage to facilities during normal operation or under any single-point fault. |
| SG-2 | Any uncommanded or unsafe motion shall be detected and arrested within a bounded time via **local** safety logic (safe-mode entry ≤ 100 ms of the triggering decision, TR-P4-020). |
| SG-3 | The robot shall enter a verified safe-mode under defined fault conditions — zero-torque/brake posture, continued heartbeat telemetry — and exit only on an explicit authenticated recovery command (TR-P4-021). |
| SG-4 | Contact-rich tasks shall stay within force/torque limits that prevent damage to canonical fixtures (connectors, fasteners, panels). |

## 2. Argument structure

**Safety architecture.** Hard limits, e-stop, and safe-mode are implemented on-robot, independent of the ground segment, and do not depend on non-critical services — high-level perception, network links, mission-control connectivity (TR-P4-022, enforced today: `trion-runtime` has no perception/network dependencies). FDIR covers all critical subsystems (actuators, power, compute, comms, perception) per [FDIR_MATRIX.md](FDIR_MATRIX.md); every service is heartbeat-supervised.

**Command authority.** Role-based command classes, two-step arming for hazardous commands, authenticated and tamper-evidently logged (TR-P4-050, TR-P4-031); the task executor checks preconditions before every step (H-08).

**Hazard coverage.** Every high-RAC hazard in the PHA maps to at least one requirement, at least one FDIR entry, and test cases in [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md).

**Verification.** Sim-first fault-injection campaigns for all high-RAC hazards; staged hardware escalation from teleop to supervised autonomy; reliability campaigns for flagship skills; post-mission reports capture anomalies, FDIR actions, and lessons learned.

## 3. Claims & evidence

| Claim | PHA hazards | Requirements | Evidence — current | Evidence — planned |
| --- | --- | --- | --- | --- |
| C-1: No catastrophic motion under a single fault | H-01, H-06, H-09, H-12 | TR-P4-012, TR-P4-020, TR-P4-022 | Escalation-to-safe-mode verified in `trion-runtime` tests (walking skeleton) | Hardware e-stop tests; full fault-injection campaign |
| C-2: Safe-mode entry within 100 ms of a critical fault | H-01, H-06 | TR-P4-020 | Safe-mode state machine + explicit-recovery-only verified in tests | Timed end-to-end measurement incl. actuator zero-torque path (sim + hardware) |
| C-3: Contact forces bounded during manipulation | H-02, H-07 | TR-P4-042c + per-skill force specs | — | Force-limited skill tests on mock fixtures (T-SIM-04 / T-HW-02) |
| C-4: Comms loss never produces unsafe motion | H-10 | TR-P4-043 | — | Comms-drop-during-contact tests (T-SIM-05) |

## 4. Assumptions & limitations

- Operations occur within a low-latency analog ground segment (line-of-sight or local network).
- Worksites are mapped; fixtures conform to the canonical designs in [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md).
- Personnel are trained in e-stop use and emergency procedures; keep-out zones are enforced during L2+ operation.
- Hardware space-rating is explicitly out of scope.

## 5. Maintenance

The safety case is updated when new hazards are identified (new tools, worksites, sensors), when a skill is promoted up the autonomy ladder, and after any incident or near-miss. Each Trion runtime/skills release includes a safety-case summary and links to its evidence log.
