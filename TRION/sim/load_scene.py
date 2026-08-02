#!/usr/bin/env python3
"""Load a Trion worksite scene and step physics.

Phase 0 acceptance check (NEXT_STEPS_AUDIT.md action 3): "a script can load
the scene and step physics." Exit code 0 means the scene is valid and stable.

Usage:
    python load_scene.py                     # headless, 1000 steps
    python load_scene.py --steps 5000
    python load_scene.py --scene worksites/w1_panel.xml --view
"""

import argparse
import sys
from pathlib import Path

import mujoco


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--scene",
        default=str(Path(__file__).parent / "worksites" / "w1_panel.xml"),
        help="MJCF scene file (default: worksites/w1_panel.xml)",
    )
    parser.add_argument("--steps", type=int, default=1000, help="physics steps to run")
    parser.add_argument("--view", action="store_true", help="open the interactive viewer")
    args = parser.parse_args()

    model = mujoco.MjModel.from_xml_path(args.scene)
    data = mujoco.MjData(model)
    if model.nkey > 0:
        mujoco.mj_resetDataKeyframe(model, data, 0)

    if args.view:
        import mujoco.viewer as mj_viewer

        mj_viewer.launch(model, data)
        return 0

    for _ in range(args.steps):
        mujoco.mj_step(model, data)

    plug_id = mujoco.mj_name2id(model, mujoco.mjtObj.mjOBJ_BODY, "plug")
    plug_pos = data.xpos[plug_id]
    handle_angle = float(data.qpos[model.jnt_qposadr[
        mujoco.mj_name2id(model, mujoco.mjtObj.mjOBJ_JOINT, "handle_hinge")
    ]])

    print(f"scene:  {args.scene}")
    print(f"bodies: {model.nbody}  joints: {model.njnt}  sim time: {data.time:.2f} s")
    print(f"handle angle: {handle_angle:.4f} rad  plug position: "
          f"({plug_pos[0]:.3f}, {plug_pos[1]:.3f}, {plug_pos[2]:.3f})")

    # Stability check: the free plug must still be on the tray, not exploded
    # off into space or fallen through the floor.
    if not (0.2 < plug_pos[0] < 0.5 and 0.7 < plug_pos[2] < 1.0):
        print("FAIL: plug left the tray region — scene is unstable", file=sys.stderr)
        return 1
    print("OK: scene loads and steps stably")
    return 0


if __name__ == "__main__":
    sys.exit(main())
