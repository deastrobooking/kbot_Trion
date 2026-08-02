# Canonical Worksites & Tasks — v0

The fixed set of worksites and tasks that drive skill design, sim scenarios, and campaign metrics (GITAI ISAM / Dextre ORU model). Adding a new task is a reviewed change; skills exist to serve these tasks, not the other way around.

## Worksites

| ID | Worksite | Contents | Exercises |
| --- | --- | --- | --- |
| **W1** | Exterior panel mockup | Handles, latches, electrical connectors, captive fasteners | Inspection, actuation, mate/demate, torque |
| **W2** | Ground-station rack | ORU-style slide-in modules, torque points, status LEDs, labels | Module swap, tool use, visual status reading |
| **W3** | Cable/umbilical interface | Connector pairs, strain reliefs, cable routing guides | Two-handed manipulation, routing, mate/demate under compliance |

W1 is the Phase 0 digital-twin scene; W2 and W3 come online in Phases 2–3.

## Canonical tasks

| ID | Task | Worksite | Skills used | Autonomy target (Phase 3) | Key hazards |
| --- | --- | --- | --- | --- | --- |
| T-01 | Visual inspection of a panel: survey, geotagged imagery, anomaly flags, before/after diff | W1/W2 | navigate, camera-survey | L4 | H-04 |
| T-02 | Actuate a handle / latch | W1 | approach, grasp, actuate | L3 | H-02 |
| T-03 | Mate / demate an electrical connector | W1/W3 | approach, grasp, insert (force-guarded) | L3 | H-02, H-05 |
| T-04 | Torque a fastener to spec | W1/W2 | tool-use, torque-sequence (scripted, deterministic) | L3 | H-02, H-06 |
| T-05 | Swap an ORU-style module: release, remove, stow, insert replacement, verify | W2 | grasp, extract, stow, insert, verify | L3 | H-02, H-03 |
| T-06 | Tool pickup and use (torque driver) | W2 | tool-acquire (end-effector interface standard), tool-use | L3 | H-03 |
| T-07 | Anomaly response: inspect → detect out-of-spec condition → propose repair plan for approval | any | T-01 + plan-proposal | L4 (proposal), execution gated | all |

## Per-task artifacts (required before hardware execution)

Each task gets, under `TRION/procedures/T-0x/`:

1. **Procedure** — steps, preconditions, abort conditions, human gates (procedures-as-code, versioned, reviewed).
2. **Skill mapping** — which `trion-skills` entries at which autonomy level ([AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md)).
3. **Sim scenario** — digital-twin setup + the FDIR fault injections this task must survive ([FDIR_MATRIX.md](FDIR_MATRIX.md)).
4. **Metrics definition** — success criteria, time budget, force limits.

## Campaign metrics (per task, per release)

- Success rate (sim and hardware, reported separately)
- Mean time to complete
- Faults encountered and FDIR actions taken
- Safety interventions (target: zero) → feeds mean-cycles-between-interventions
