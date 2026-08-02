# TRION/sim — Digital twin & fault-injection harness (P6)

Simulation assets for the Trion space operations engineer. Worksite scenes are plain MJCF, kept separate from the robot model so scenes stay modular and composable.

## Layout

```
sim/
├── load_scene.py        # load + step a scene; Phase 0 acceptance check
└── worksites/
    └── w1_panel.xml     # Worksite W1: panel with handle (T-02), latch,
                         # connector socket + free plug (T-03)
```

## Running

MuJoCo Python bindings are required (`pip install mujoco`; use `mujoco<3.2` on Python 3.9):

```bash
python sim/load_scene.py                 # headless smoke test, exit 0 = stable
python sim/load_scene.py --view          # interactive viewer
python sim/load_scene.py --steps 5000
```

## Scene conventions

- Each task target carries a **site** for skills/tests to reference: `handle_grip_site`, `latch_tab_site`, `socket_entry_site`, `plug_grip_site`.
- Articulated fixtures use named joints with realistic ranges and springs: `handle_hinge` (0–90°, sprung closed), `latch_slide` (0–4 cm, sprung).
- Every scene defines a `home` keyframe as the canonical start state for campaigns.
- Worksites W2 (ground-station rack) and W3 (cable interface) follow in Phases 2–3 per [WORKSITES_AND_TASKS.md](../docs/WORKSITES_AND_TASKS.md).

## K-Bot integration (next)

The robot model comes from `kscale-assets` (a nested submodule of `ksim-kbot`, pulled via `git submodule update --init --recursive` inside `ksim-kbot/`). The composition step — K-Bot + worksite in one scene via MJCF `<include>`/attach — is the remaining Phase 0 digital-twin work, followed by the fault-injection harness (kill services, drop IMU, spike temperatures mid-task) that exercises the FDIR matrix per release (TR-P4-013).
