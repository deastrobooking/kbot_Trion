# Upstream K-Bot Code Audit — Features, Drivers & Improvement Opportunities

**Date:** 2026-08-02  
**Scope:** `kos-kbot/`, `kbot-inference/`, `ksim-kbot/` (upstream submodules).  
**Auditor:** GitHub Copilot  
**Constraint:** Per [AGENT_GUIDE.md](../AGENT_GUIDE.md), findings are documented in `TRION/`; no upstream files were modified.

---

## Executive summary

The upstream stack is a credible alpha-stage humanoid platform: Rust services for actuators/IMU/video/power, a standalone ONNX inference runner, and a JAX/MuJoCo training pipeline. For Trion's mission-critical space-operations goals, the biggest gaps are not in any single driver but in *system-level integration*: there is no safety/FDIR layer, no E-stop path, no perception, no manipulation/end-effector model, no worksite simulation, and no shared command authority between `kos-kbot` and `kbot-inference`. The two Rust runtimes (`kos-kbot` and `kbot-inference`) also duplicate actuator configuration and IMU logic, which creates a maintenance and safety risk.

This audit groups findings by subsystem, rates severity, and maps each item to the Trion pillar or phase where it should be addressed.

---

## 1. Architecture & integration findings

### 1.1 Two independent control paths compete for the same hardware

| Aspect | Finding | Impact |
|--------|---------|--------|
| `kos-kbot` | KOS gRPC service layer; owns actuators, IMU, process manager, video, power board. | The "official" robot OS. |
| `kbot-inference` | Standalone binary that talks directly to `robstride` / `hiwonder` crates, bypassing KOS. | Used to run ONNX policies on hardware. |

Both crates create their own `Supervisor`/CAN transports and their own IMU readers. On a real robot, running `kbot-inference` while `kos-kbot` is active will cause **bus contention** on CAN and serial ports. This is a safety-critical conflict.

**Recommendation:**
- Long term: `kbot-inference` should consume KOS gRPC services (via `pykos` or a Rust KOS client), not direct hardware crates.
- Short term: document mutual exclusion and add a runtime lock/mutex (e.g., a systemd service conflict or a hardware arbitration crate) so only one controller owns the actuators at a time.
- Trion phase: Phase 1/P1 — command authority layer must gate which process can command actuators.

### 1.2 No safety/FDIR integration in upstream stack

The upstream code has no concept of:
- Safe-mode / zero-torque posture
- E-stop
- Watchdog escalation
- Command authentication or command classes
- Frame-overrun detection
- Health telemetry bus

The `trion-runtime` crate in `TRION/src/trion-runtime/` is designed to fill this gap, but it currently does not interface with upstream services.

**Recommendation:**
- Define a Trion *safety shim* that wraps/observes `kos-kbot` services (gRPC health + a CAN-level heartbeat) and can command safe-mode entry.
- Expose `ModeController` from `trion-runtime` as a KOS telemetry topic so ground control can display run mode.
- Trion phase: Phase 0/P4 — continue; target Phase 1 exit criteria.

### 1.3 No perception or manipulation stack

Upstream is purely *locomotion-centric*:
- `ksim-kbot` tasks are standing, walking, jumping, get-up.
- No camera-based localization, no object detection, no worksite mapping.
- No end-effector model; the wrist actuators are treated like any other joint.
- No force/torque sensing abstraction beyond actuator feedback.

For Trion's ISAM tasks (handle actuation, connector mating, ORU swap), this is a hard blocker.

**Recommendation:**
- Add `TRION/src/trion-perception` for vision-based localization and fixture recognition.
- Define an end-effector/tool interface in `TRION/src/trion-skills` even before a physical gripper exists.
- Trion phase: Phase 2/P2–P3.

---

## 2. Driver & hardware findings

### 2.1 Actuator driver (`kos-kbot/src/actuator.rs`)

#### 2.1.1 Hardcoded defaults and missing configuration source

[actuator.rs#L67-L74](kos-kbot/src/actuator.rs#L67) maps actuator faults inline in `get_actuators_state`. The comment at [L173](kos-kbot/src/actuator.rs#L173) notes this should be moved to avoid async slowdowns. More importantly, there is no external source of truth for:
- Joint limits
- KP/KD per joint
- Safe torque/current limits
- Command rate limits

Everything is wired in [lib.rs#L192-L433](kos-kbot/src/lib.rs#L192) as a large literal array. Changing a robot configuration requires a recompile.

**Recommendation:**
- Load actuator configuration from a versioned YAML/JSON file (e.g., `TRION/config/kbot_actuator_config.yaml`).
- Validate at startup that discovered motors match the config; refuse to enable torque on mismatch.
- Trion phase: Phase 0/P4 — configuration audit trail is part of the safety case.

#### 2.1.2 No command validity checks before sending

`command_actuators` converts optional fields to `0.0` when absent ([actuator.rs#L88-L95](kos-kbot/src/actuator.rs#L88)) and forwards them without checking joint limits, velocity limits, or collision constraints. There is no interpolation guard if the command jumps beyond `max_angle_change`.

**Recommendation:**
- Add a command sanity gate in the actuator service or in `trion-runtime` that clamps/rejects commands outside configured limits.
- Trion phase: Phase 1/P1 — shared control must enforce force/position limits.

#### 2.1.3 `found_motors` logic silently tolerates missing actuators

A configured motor that is not discovered only logs a warning ([actuator.rs#L62-L68](kos-kbot/src/actuator.rs#L62)). The service then continues, which can cause later commands to fail or, worse, move the robot with an incomplete kinematic chain.

**Recommendation:**
- Make missing critical actuators a startup error (configurable per actuator).
- Expose missing motors in the health bus.
- Trion phase: Phase 0/P4.

### 2.2 IMU drivers (`kos-kbot/src/hexmove.rs`, `hiwonder.rs`)

#### 2.2.1 Hexmove driver is compiled out

`hexmove.rs` exists but is commented out in [lib.rs#L9-L12](kos-kbot/src/lib.rs#L9). Only `hiwonder.rs` is active. If the robot uses Hexmove IMUs, the current build will not support them.

**Recommendation:**
- Re-enable `hexmove` behind a config flag or auto-detect at runtime.
- Trion phase: Phase 0/P4 — IMU redundancy is part of FDIR (F-05).

#### 2.2.2 `get_advanced_values` returns all `None`

Both IMU implementations return empty `ImuAdvancedValuesResponse` ([hexmove.rs#L83-L94](kos-kbot/src/hexmove.rs#L83), [hiwonder.rs#L82-L93](kos-kbot/src/hiwonder.rs#L82)). Temperature, linear acceleration, and gravity vector are not exposed, even though the underlying `hiwonder` crate appears to provide accelerometer/gyro/quaternion data.

**Recommendation:**
- Populate advanced values from available sensor data; temperature is needed for FDIR F-03 thermal monitoring.
- Trion phase: Phase 1/P4.

#### 2.2.3 No IMU dropout detection

The IMU service reads on demand. If the sensor stops responding, callers just get stale data or errors. There is no internal watchdog marking the IMU as offline.

**Recommendation:**
- Add a background health task that timestamps every successful read and publishes `Degraded`/`Critical` after a threshold.
- Trion phase: Phase 0/P4 — FDIR F-05.

### 2.3 Power board (`kos-kbot/src/lib.rs#L32-L104`)

#### 2.3.1 Power board is disabled at compile time

`USE_POWERBOARD` is `false` ([lib.rs#L32](kos-kbot/src/lib.rs#L32)). The initialization code uses `expect` in the blocking task ([lib.rs#L63](kos-kbot/src/lib.rs#L63)), which violates the upstream "no unwrap/expect" rule.

**Recommendation:**
- Make `USE_POWERBOARD` a runtime config, not a constant.
- Replace `expect` with `eyre` error propagation or a logged fatal path.
- Trion phase: Phase 0/P4 — power monitoring is FDIR F-08.

#### 2.3.2 Power board data is not exposed as a structured health topic

Frames are published under `powerboard/general` and `powerboard/limbs`, but voltage/current/power are not compared against thresholds.

**Recommendation:**
- Add FDIR monitors for undervoltage, overcurrent, and per-limb power anomalies.
- Trion phase: Phase 1/P4.

### 2.4 Video / process manager (`kos-kbot/src/process_manager.rs`)

#### 2.4.1 Hardcoded camera device and parameters

`/dev/video47`, 1280×1080 @ 30 fps, YUY2, vertical flip are hardcoded ([process_manager.rs#L48-L80](kos-kbot/src/process_manager.rs#L48)). There is no fallback for a missing camera.

**Recommendation:**
- Move camera config to a config file; support device enumeration and graceful degradation to "no video".
- Trion phase: Phase 1/P5 — ground console needs predictable video behavior.

#### 2.4.2 GStreamer pipeline may leak on stop

`stop_kclip` sends EOS and waits up to 2 s, then forces `Null`. If the pipeline is in a bad state, mutexes may be held indefinitely.

**Recommendation:**
- Add a timeout around the whole stop operation and an `Abort` path.
- Trion phase: Phase 1/P4 — stuck video must not block safe-mode entry.

#### 2.4.3 `combine_with_video` uses `unwrap` on paths

[lib.rs#L393-L395](kos-kbot/src/lib.rs#L393) calls `.to_str().unwrap()` on paths that were just constructed and are valid UTF-8, but this still violates the project's no-unwrap rule and is brittle if paths ever come from user input.

**Recommendation:**
- Use `ok_or_else`/`?` or `as_os_str` with the `krec` API.
- Trion phase: Phase 0 (code-quality fix).

---

## 3. Inference & control findings

### 3.1 `kbot-inference` bypasses KOS

As noted in §1.1, `kbot-inference` duplicates hardware initialization. It also hardcodes its own actuator ID map, home position, joint limits, and KP/KD tables in [constants.rs](kbot-inference/src/constants.rs).

**Recommendation:**
- Unify configuration with `kos-kbot` via a shared config file or a KOS gRPC query at startup.
- Trion phase: Phase 1.

### 3.2 Inference runner has no safety limits

[nn.rs#L261-L282](kbot-inference/src/nn.rs#L261) clips outputs to `NN_JOINT_LIMITS`, but those limits are training limits, not runtime safety limits. There is no:
- Velocity/torque ramp limit
- Joint limit independent of NN output
- E-stop input
- Safe-mode hook

**Recommendation:**
- Insert a Trion safety guard between NN output and actuator command that can override with zero torque / safe posture.
- Trion phase: Phase 1/P4.

### 3.3 Observation construction is fragile

`update_observation` assumes fixed array slices ([nn.rs#L289-L303](kbot-inference/src/nn.rs#L289)). If the model input size changes, the code will panic or silently misalign. There is no schema validation against the ONNX model.

**Recommendation:**
- Read input/output shapes from the ONNX session at load time and validate against the configured observation/action spec.
- Trion phase: Phase 1/P4 — deterministic control loop depends on known tensor shapes.

### 3.4 Loop timing is open-loop

[run_model.rs#L163-L200](kbot-inference/src/bin/run_model.rs#L163) computes a target loop interval but does not bound jitter or trigger safe-mode on repeated overruns.

**Recommendation:**
- Add a WCET monitor and frame-overrun policy per TR-P4-060.
- Trion phase: Phase 2/P4.

### 3.5 `expect` and `unwrap` in inference crate

Found:
- [lib.rs#L13](kbot-inference/src/lib.rs#L13) `expect("Setting default subscriber failed")`
- [bin/run_model.rs#L48](kbot-inference/src/bin/run_model.rs#L48) `to_str().unwrap()`
- Multiple `expect` in binaries for tracing setup.

Upstream README says "No unwrap() or expect()"; these are easy wins.

**Recommendation:**
- Replace with `eyre` or `tracing_subscriber::try_init()` and propagate errors.
- Trion phase: Phase 0 (code-quality).

---

## 4. Simulation & training findings

### 4.1 `ksim-kbot` has no manipulation or worksite tasks

All tasks are locomotion/standing. There is no:
- Fixture model (handle, latch, connector)
- End effector or gripper
- Contact-rich reward for insertion
- Multi-step task structure

**Recommendation:**
- Create `TRION/sim/` with worksite W1 and new `ksim` tasks for inspection, handle actuation, and connector mating.
- Trion phase: Phase 0/P6 (immediate next action).

### 4.2 Tests are trivial

[tests/test_dummy.py](ksim-kbot/tests/test_dummy.py) only asserts `True`. There are no unit tests for observations, rewards, or model export.

**Recommendation:**
- Add tests that instantiate tasks, step physics, and verify observation/action shapes.
- Add an ONNX-export round-trip test.
- Trion phase: Phase 0/P6.

### 4.3 `common.py` has TODO-quality reward names

`DHForwardReward`, `DHControlPenalty`, `DHHealthyReward` reference "DH" (likely "default humanoid") but are used for K-Bot. Naming should be robot-specific to avoid confusion.

**Recommendation:**
- Rename to `KBotForwardReward`, etc., or document the legacy naming.
- Trion phase: Phase 0 (code-quality).

### 4.4 Deploy scripts use TensorFlow SavedModel, not ONNX

[deploy/sim.py](ksim-kbot/ksim_kbot/deploy/sim.py) and [deploy/real.py](ksim-kbot/ksim_kbot/deploy/real.py) load `tf.saved_model`, while `kbot-inference` loads ONNX. This is a deployment impedance mismatch.

**Recommendation:**
- Standardize on ONNX as the deployment artifact (already used by `kbot-inference`).
- Provide an export script that produces the ONNX file consumed by `kbot-inference`.
- Trion phase: Phase 1/P6.

### 4.5 Deploy scripts duplicate actuator metadata

`ACTUATOR_LIST` in `sim.py` and `real.py` duplicates joint names, IDs, KP/KD, and max torque. These should come from a single source of truth.

**Recommendation:**
- Generate `ACTUATOR_LIST` from the same config file proposed for `kos-kbot`.
- Trion phase: Phase 1.

---

## 5. Configuration & operational findings

### 5.1 Hardcoded device paths everywhere

| File | Hardcoded path |
|------|----------------|
| `kos-kbot/src/lib.rs` | `/dev/ttyUSB0`, `can0`–`can4` |
| `kos-kbot/src/process_manager.rs` | `/dev/video47` |
| `kbot-inference/src/lib.rs` | `/dev/ttyUSB0`, `/dev/ttyCH341USB0`, `can0`–`can4` |

These will differ across robot variants and OS/driver enumerations. They should be configuration, not source code.

**Recommendation:**
- Move all device paths to a runtime config file or environment variables with documented defaults.
- Trion phase: Phase 0/P4.

### 5.2 No versioning of robot configuration

There is no manifest that says "this robot has 20 actuators, IDs 11–15/21–25/31–35/41–45, with these gains." The config is scattered across Rust literals and Python lists.

**Recommendation:**
- Create a `TRION/config/robot_manifest.yaml` that is consumed by Rust, Python, and the sim.
- Trion phase: Phase 0/P4.

### 5.3 Serial number is a placeholder

[lib.rs#L157](kos-kbot/src/lib.rs#L157) returns `"00000000"`. This breaks telemetry attribution and safety-case traceability.

**Recommendation:**
- Read serial from a hardware identifier or a config file; fail startup if unknown.
- Trion phase: Phase 1/P5.

---

## 6. Prioritized improvement backlog

### P0 — fix before Phase 0 close

1. **Unify actuator/IMU configuration** into a single config file consumed by `kos-kbot`, `kbot-inference`, `ksim-kbot/deploy`, and `trion-runtime`.
2. **Add TRION CI** that builds and tests `kos-kbot`/`kbot-inference` with clippy (no unwrap/expect) and runs `ksim-kbot` unit tests.
3. **Create `TRION/sim/`** with MuJoCo worksite W1.
4. **Document hardware mutual exclusion** between `kos-kbot` and `kbot-inference`.

### P1 — Phase 1 blockers

5. **Implement Trion safety shim** over KOS services: E-stop, safe-mode, command authority, actuator command gate.
6. **Add IMU health monitoring** and actuator fault escalation to the health bus.
7. **Replace hardcoded device paths** with config-driven initialization.
8. **Standardize deployment artifact** on ONNX with an export script.
9. **Add real serial number / robot manifest**.

### P2 — Phase 2 readiness

10. **Add perception crate** with camera integration, localization, and fixture recognition.
11. **Add manipulation skill crate** with end-effector abstraction and force-aware control.
12. **Implement full FDIR matrix** (F-02 through F-08) in `trion-runtime`.
13. **Add frame-overrun monitoring** and WCET measurement in the control loop.

### P3 — Phase 3+ extensions

14. **Train manipulation policies** in `ksim` for handle, latch, connector tasks.
15. **Add multi-step task executor** with preconditions, abort conditions, and human go/no-go gates.
16. **Build ground-control console** consuming KOS telemetry and commanding via Trion command authority.

---

## 7. Quick-win code-quality issues

| Location | Issue | Fix |
|----------|-------|-----|
| `kos-kbot/src/lib.rs#L63` | `expect` in power board runtime | Return error or log fatal |
| `kos-kbot/src/lib.rs#L393-L395` | `unwrap` on path conversion | Use `?` error handling |
| `kbot-inference/src/lib.rs#L13` | `expect` on tracing init | Use `try_init` and propagate |
| `kbot-inference/src/bin/run_model.rs#L48` | `unwrap` on model path | Validate with `eyre` |
| Multiple binaries | `expect` on tracing init | Same as above |
| `ksim-kbot/tests/test_dummy.py` | Trivial test | Add real shape tests |

---

## 8. Conclusion

The upstream K-Bot code is a reasonable starting point for a locomotion-focused research humanoid, but it is not yet a mission-critical operations platform. The highest-leverage improvements for Trion are:

1. **Safety/FDIR integration** — wrap upstream services with `trion-runtime`.
2. **Single source of truth for robot configuration** — eliminate duplicated actuator/IMU metadata.
3. **Simulation-first V&V** — build `TRION/sim/` with worksite W1 and fault injection.
4. **Perception and manipulation stack** — add the missing P2/P3 layers.

These findings are consistent with the risks already identified in [PROJECT_PLAN.md](PROJECT_PLAN.md): upstream alpha churn, sim-to-real gap, and safety-logic entanglement. Addressing them in `TRION/` (without modifying submodules) preserves the upstream-diff cleanliness required by [AGENT_GUIDE.md](../AGENT_GUIDE.md).
