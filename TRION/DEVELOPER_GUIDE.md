# Trion Developer Guide — K-Bot Architecture & Rust

Trion is our experimental layer on top of the upstream [K-Bot](https://github.com/kscalelabs/kbot) humanoid robot from K-Scale Labs. This guide explains what K-Bot actually is under the hood, how Rust-native the stack is, and how the pieces fit together. For git workflow, branching, and submodule mechanics, see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## Is this a Rust robotics framework? (Yes — mostly)

The umbrella repo looks like "just docs" because the software lives in **git submodules** — the folders are empty until you run `git submodule update --init`. Once initialized, the real code appears, and the language split (measured from the actual repos) is:

| Component         | Languages                  | Role                                        |
| ----------------- | -------------------------- | ------------------------------------------- |
| `kos-kbot/`       | ~50% Rust / ~50% Python    | The robot's runtime OS layer (core is Rust; Python is test/utility scripts) |
| `kbot-inference/` | ~65% Rust / ~35% Python    | On-robot neural-net policy inference (Rust binary running ONNX) |
| `ksim-kbot/`      | ~100% Python               | RL training environments (JAX/MuJoCo via K-Scale's `ksim`) |
| `kos` (upstream dep, not vendored here) | ~55% Rust / ~40% Python | K-Scale OS core framework — the actual "Rust robotics framework" |

**The verdict:** everything that runs *on the robot* — hardware drivers, the OS service layer, real-time inference — is Rust. Everything used to *train* the robot — simulation, RL — is Python/JAX. That split is deliberate: Rust for reliability and low latency on embedded hardware (the robot's compute is an aarch64 Linux board), Python for ML research velocity.

## The stack, top to bottom

```
┌────────────────────────────────────────────────────┐
│  ksim-kbot (Python/JAX)                            │
│  Train locomotion/standing policies in simulation  │
│  → exports policy as ONNX file                     │
└───────────────────────┬────────────────────────────┘
                        │  .onnx policy file
┌───────────────────────▼────────────────────────────┐
│  kbot-inference (Rust)                             │
│  Loads ONNX via `ort` (ONNX Runtime), runs the     │
│  control loop: IMU + joint states in → torques out │
└───────────────────────┬────────────────────────────┘
                        │  gRPC / direct crate calls
┌───────────────────────▼────────────────────────────┐
│  kos-kbot (Rust) — implements KOS for this robot   │
│  Hardware abstraction: actuators, IMU, power       │
│  board, cameras, process manager                   │
└───────────────────────┬────────────────────────────┘
                        │  CAN bus / serial / USB
┌───────────────────────▼────────────────────────────┐
│  Physical K-Bot: Robstride actuators, Hiwonder/    │
│  Hexmove IMUs, power board, cameras                │
└────────────────────────────────────────────────────┘
```

### KOS — the framework itself

[KOS](https://github.com/kscalelabs/kos) (K-Scale OS) is the actual Rust robotics framework. It defines gRPC services for robot capabilities (actuators, IMU, video, process management), and robot-specific crates like `kos-kbot` implement those services for real hardware. Client code (Python or anything gRPC-speaking) talks to the robot through these services. It's consumed here as a crates.io dependency (`kos = "0.7.4"`), not a submodule.

### kos-kbot — the K-Bot hardware layer

The Rust sources (in `kos-kbot/src/`) map directly to hardware subsystems:

- [actuator.rs](../kos-kbot/src/actuator.rs) — joint control via the `robstride` crate (CAN-bus servo actuators)
- [hexmove.rs](../kos-kbot/src/hexmove.rs) / [hiwonder.rs](../kos-kbot/src/hiwonder.rs) — IMU drivers (via the `imu` crate, Linux-only target deps)
- [process_manager.rs](../kos-kbot/src/process_manager.rs) — service lifecycle, video via GStreamer
- `scripts/` — Python utilities for bring-up and testing (`move_motor.py`, `read_imu.py`, `walk.py`, `zero_in_place.py`, …) — this is where most of the repo's Python lives

Notable crates it pulls in: `robstride` (actuators), `imu`/`hiwonder` (IMUs), `kbot-pwrbrd` (power board), `krec` (telemetry recording), `tokio` (async runtime), `gstreamer` (camera pipelines).

### kbot-inference — the control loop

A Rust binary (crate name `kbot`) that loads a trained policy (`position_control.onnx` ships in the repo) with the `ort` ONNX Runtime bindings, reads IMU + actuator state, and emits position/torque commands. Uses `tonic` (gRPC), `ndarray` (tensor math), `clap` (CLI). Same hardware crates (`robstride`, `hiwonder`, `kbot-pwrbrd`) on Linux targets.

### ksim-kbot — training

Pure Python. Defines K-Bot RL tasks on K-Scale's `ksim` framework (JAX-based, MuJoCo physics): `standing/` (MLP and LSTM variants), `misc_tasks/`, and `deploy/` (`sim.py`/`real.py` for running trained policies). Robot meshes/assets come from a nested `kscale-assets` submodule. Trained policies export to ONNX for the Rust inference stack — that ONNX file is the contract between the Python world and the Rust world.

## Building the Rust components

Prerequisites: a Rust toolchain (`rustup`), and [`cross`](https://github.com/cross-rs/cross) for targeting the robot.

```bash
# native build (stub hardware features — fine on macOS for compile checks)
cd kos-kbot && cargo build

# cross-compile for the robot's onboard computer
cross build --release --target aarch64-unknown-linux-gnu

# run with logging
RUST_LOG=debug cargo run
```

Note the Linux-gated dependencies (`[target.'cfg(target_os = "linux")'.dependencies]` in both Cargo.tomls): real IMU/actuator drivers only compile on Linux. On macOS you build against stubs — good for development, but hardware code paths need the robot (or a Linux box) to exercise.

Upstream Rust conventions (hold Trion Rust code to the same bar):

- `cargo fmt --all`, `cargo clippy`, `cargo test` before committing
- `tracing` for logging, `eyre` for error handling
- **No `unwrap()` or `expect()`**

## Setting up ksim-kbot (training)

```bash
cd ksim-kbot
pip install -e .                              # Python 3.11
git submodule update --init --recursive       # pulls kscale-assets
```

See the [ksim docs](https://docs.kscale.dev/docs/ksim) for training and debugging workflows. Note: JAX training is realistically a Linux + NVIDIA GPU workflow.

## Hardware side

- `mechanical/` — CAD is on [Onshape](https://cad.onshape.com/publications/e15cf8edefacbba3009917c0/); design goals are repairability, mass-manufacturability, and modularity (swappable hands via lens-mount, swappable USB-device head).
- `electrical/` — points to the [K-Scale electrical docs](https://docs.kscale.dev/robots/k-bot/electrical/). Arms/hands share 48 V power and CAN bus lines.

## Where Trion fits

Our code lives in `TRION/` and should *consume* the stack rather than fork it:

- **Talk to the robot** through KOS gRPC services (any language) or by depending on the published crates (`kos`, `robstride`, etc.) from Rust code in `TRION/src/`.
- **New training tasks** can live in `TRION/` as a package that imports `ksim`, mirroring `ksim_kbot`'s structure.
- **Patches to K-Scale code** go upstream (see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md)) — not into the submodules.

Suggested layout as the folder grows:

```
TRION/
├── DEVELOPER_GUIDE.md   # this file
├── AGENT_GUIDE.md       # rules for AI agents in this repo
├── docs/                # CONTRIBUTING.md, design notes, experiment writeups
├── src/                 # Trion feature code (Rust crate or Python package)
├── scripts/             # tooling and automation
└── experiments/         # prototypes (scratch/ is git-ignored)
```

## Licensing

The umbrella repo: `CERN-OHL-S` (hardware) / `GPL v3` (software) per the root `LICENSE` files. The submodules themselves are **MIT-licensed** (kos-kbot, ksim-kbot). Trion code that only *uses* MIT crates can be licensed freely; anything derived from GPL-covered parts of the umbrella inherits GPL v3.
