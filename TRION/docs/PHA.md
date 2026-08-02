# Preliminary Hazard Analysis (PHA) & Safety Case Skeleton — v0

Modeled on NASA/ISS practice (Robonaut 2 lessons learned): safety is priority one, safety systems are verified meticulously, and safety logic is separated from control logic and kept local to the robot. This document feeds [FDIR_MATRIX.md](FDIR_MATRIX.md) and [REQUIREMENTS.md](REQUIREMENTS.md).

Severity: **Cat-1** injury to a person · **Cat-2** damage to robot or worksite · **Cat-3** mission loss/abort only.
Likelihood (pre-mitigation, engineering judgment at v0): H / M / L.

## Hazard table

| ID | Hazard | Sev | Lik | Causes | Mitigations | Verified by |
| --- | --- | --- | --- | --- | --- | --- |
| H-01 | Uncontrolled motion near humans or equipment | Cat-1 | M | Software fault, runaway policy output, stale state estimate, comm-injected command | Safe-mode within 100 ms (TR-P4-020); watchdog + FDIR (TR-P4-010…013); joint velocity/torque hard limits enforced below the policy layer; e-stop (hardware + immediate command class TR-P4-042a); safety/control separation (TR-P4-022) | Timed safe-mode tests; fault-injection campaign (F-01, F-05, F-06, F-09) |
| H-02 | Excessive contact force at worksite (crushed connector, damaged panel) | Cat-2 | H | Bad grasp/insertion policy, missing force feedback, wrong worksite model | Force/torque limits per skill; commit-window approval before contact (TR-P4-042c); Dextre-style force-moment monitoring with abort thresholds | Skill-level force-limit tests in sim + hardware |
| H-03 | Dropped tool or module | Cat-2 | M | Grasp failure, unexpected disturbance | Grasp verification (force + vision) before transport; tool/module retention checklist in task plans; drop detection event | Task sim scenarios with grasp-failure injection |
| H-04 | Fall / tip-over (legged operation) | Cat-2 | M | Locomotion policy failure, unexpected contact, floor conditions | Mounted-torso mode for manipulation-heavy tasks (Apollo pattern); locomotion policies trained with disturbance injection; fall-detection → zero-torque crouch | Locomotion campaign metrics; fall-response test |
| H-05 | Electrical hazard (48 V bus, CAN wiring) during servicing | Cat-1 | L | Miswired worksite mockup, insulation damage, robot bridging contacts | Worksite electrical review; insulated end-effector surfaces; power-off procedures for electrical tasks encoded as plan preconditions | Procedure review; inspection checklist |
| H-06 | Actuator thermal runaway | Cat-2 | M | Sustained overtorque, blocked joint, cooling failure | Overtemp detection + torque derating (F-02); duty-cycle limits in skills | Fault injection (F-02); hardware soak test |
| H-07 | Pinch points during shared-control or autonomous motion | Cat-1 | M | Operator error, bystander proximity | Keep-out zone around robot during L2+ operation; speed reduction near humans; predictable motion (no un-commanded reposturing) | Ops procedure; human-factors review |
| H-08 | Unauthorized or spoofed command | Cat-1 | L | Unsecured ground link, credential leak | Command authentication + role-based classes + two-step arming (TR-P4-050); tamper-evident logs (TR-P4-031) | Security review + penetration test |

## Safety case skeleton

Top-level safety goals, each mapped to requirements and evidence. This structure becomes the Phase 4 safety case v1.

| Goal | Claim | Supported by | Evidence (to accumulate) |
| --- | --- | --- | --- |
| G1 | The robot never moves uncontrolled near humans or equipment | H-01 mitigations; TR-P4-020/022/042a; hard limits below policy layer | Timed safe-mode test results; fault-injection campaign logs; e-stop test records |
| G2 | Contact forces at the worksite are always bounded | H-02 mitigations; per-skill force limits; commit windows | Per-skill force-limit test data (sim + hardware) |
| G3 | Any detected fault leads to a safe state within its specified bound | FDIR matrix completeness (TR-P4-013); safe-mode timing (TR-P4-020) | 100% FDIR entries exercised per release; timing measurements |
| G4 | Only authorized humans can command hazardous actions | TR-P4-050; TR-P4-031 | Security review report; command-audit logs |

**Working rule (Robonaut 2):** no capability is promoted up the autonomy ladder until the safety goals it touches have evidence at the current level. Safety first, capability second.
