# Trion Recommendations & Key Rust Rewrites

**Date:** 2026-08-02  
**Status:** Design proposals / documentation only — no code changes  
**Scope:** Consolidated improvement plan derived from:
- [NEXT_STEPS_AUDIT.md](NEXT_STEPS_AUDIT.md) — Trion project review
- [UPSTREAM_KBOT_AUDIT.md](UPSTREAM_KBOT_AUDIT.md) — upstream K-Bot code review
- [PROJECT_PLAN.md](PROJECT_PLAN.md) — canonical roadmap

**Note on current state:** As of this writing, [PROJECT_PLAN.md](PROJECT_PLAN.md) marks Phase 0 as complete (runtime skeleton, worksite W1, CI skeleton, and v0 docs are done). The items in §3 below that were originally P0 are now **verified/closed**; they are retained for reference and because the configuration-manifest work is still being promoted into Phase 1. The actionable engineering backlog begins in §4.

This document is the single source of truth for the next engineering cycle. It preserves the rule that upstream submodules remain unmodified; all implementations belong under `TRION/`.

---

## 1. Guiding principles

1. **Safety before capability.** Every new feature must pass through the Trion safety/FDIR layer before it can command motion.
2. **One source of truth for robot configuration.** Actuator IDs, gains, limits, and device paths must live in one versioned file consumed by Rust, Python, and simulation.
3. **Sim-first V&V.** Every skill and every FDIR entry must be exercised in `TRION/sim/` before hardware.
4. **Upstream stays pristine.** All changes are in `TRION/`; upstream gaps are consumed as services or documented as upstream PR candidates.
5. **Flight-software discipline.** No `unwrap`/`expect`; bounded memory after init; bounded loops; every error checked.

---

## 2. Consolidated priority matrix

| Priority | Initiative | Owner crate / path | Status | Closes |
|----------|------------|-------------------|--------|--------|
| P0 | TRION CI + build verification | `.github/workflows/trion-ci.yml` | ✅ Done | TR-P4-070, Phase 0 exit |
| P0 | MuJoCo worksite W1 | `TRION/sim/worksites/w1_panel.xml` | ✅ Worksite only | Phase 0 digital-twin fixture |
| P1 | Robot configuration manifest | `TRION/config/robot_manifest.yaml` | 🟡 Simulation manifest | Duplicated config in upstream |
| P1 | KOS service safety shim | `TRION/src/trion-runtime/src/kos_shim/` | 🔲 Open | E-stop, safe-mode, command gate |
| P1 | IMU health monitor + FDIR escalation | `TRION/src/trion-runtime/src/fdir/` | 🔲 Open | F-05 |
| P1 | Actuator command sanity gate | `TRION/src/trion-runtime/src/command/gate.rs` | ✅ Core done | Joint-limit enforcement; KOS integration remains |
| P1 | Trion policy service (ONNX via KOS) | `TRION/src/trion-policy-service/` | 🔲 Open | Replace direct-hardware inference |
| P1 | Command authority + command classes | `TRION/src/trion-runtime/src/command/authority.rs` | 🔲 Open | TR-P4-042, TR-P4-050 |
| P2 | Perception crate | `TRION/src/trion-perception/` | 🔲 Open | P2 localization |
| P2 | Skill crate + autonomy ladder | `TRION/src/trion-skills/` | 🔲 Open | P1/P3 skills |
| P2 | Full FDIR matrix (F-02…F-08) | `TRION/src/trion-runtime/src/fdir/` | 🔲 Open | P4 runtime |
| P2 | Mission-control crate | `TRION/src/mission-control/` | 🔲 Open | P5 ground segment |
| P3 | Manipulation policy training | `TRION/sim/` + `ksim-kbot` tasks | 🔲 Open | Phase 3 ISAM skills |
| P3 | Task executor + human gates | `TRION/src/trion-skills/src/executor.rs` | 🔲 Open | Phase 2/3 plans |

---

## 3. P0 — closed / retained for reference

These items were the original Phase 0 close-out actions. They are now complete in [PROJECT_PLAN.md](PROJECT_PLAN.md) and are kept here only as design reference.

### 3.1 TRION CI workflow

`.github/workflows/trion-ci.yml` exists and runs `cargo fmt`, `clippy`, and `test` for `TRION/src`, the walking-skeleton demo, and a headless W1 MuJoCo smoke test on every PR touching `TRION/**`.

**What it must do:**
```yaml
- checkout the repository
- install Rust stable + clippy + fmt
- cd TRION/src && cargo fmt --all -- --check
- cd TRION/src && cargo clippy --all-targets -- -D warnings
- cd TRION/src && cargo test
- cd TRION/src && cargo run --bin walking_skeleton (smoke test)
- python TRION/sim/load_scene.py --steps 2000 (MuJoCo smoke test)
```

**Acceptance:** PRs are blocked on formatting, clippy, or test failures. This removes dependency on any single developer's host environment.

### 3.2 Robot configuration manifest

Create `TRION/config/robot_manifest.yaml` as the single source of truth.

**Proposed schema:**
```yaml
manifest_version: "0.1.0"
robot:
  serial: "KBot2-00000000"  # override per physical unit
  model: kbot2
  actuators:
    - id: 11
      name: left_shoulder_pitch_03
      type: RobStride03
      bus: can1
      kp: 150.0
      kd: 5.0
      max_torque: 60.0
      max_velocity: 125.6
      joint_limits_deg: [0.0, 180.0]
      command_rate_hz: 100.0
      critical: true
    # ... remaining actuators
  imus:
    - name: base_imu
      driver: hiwonder
      interface: /dev/ttyUSB0
      baud_rate: 9600
      frequency_hz: 100
      critical: true
  power:
    - name: main_powerboard
      interface: can0
      enabled: true
  video:
    - name: head_camera
      device: /dev/video47
      width: 1280
      height: 1080
      fps: 30
      format: YUY2
      flip: vertical
```

**Why this matters:**
- Eliminates duplicated `ACTUATOR_LIST` in `ksim-kbot/ksim_kbot/deploy/sim.py` and `real.py`.
- Lets `kos-kbot` load config at startup instead of compiling actuator tables into `lib.rs`.
- Lets `kbot-inference` query KOS at startup instead of hardcoding its own `constants.rs`.
- Provides the traceability required by the safety case.

**Upstream PR candidate:** Provide a config loader in `kos-kbot` that accepts a path via CLI or env var; fallback to current literals for backward compatibility.

### 3.3 MuJoCo worksite W1

`TRION/sim/worksites/w1_panel.xml` exists with a loadable scene and smoke test. Remaining open work: compose the full K-Bot model into the scene and add manipulation-task definitions.

---

## 4. P1 — key Rust rewrites and new modules

These are **designs only**; they describe the Rust modules to be implemented in `TRION/src/`.

### 4.1 KOS service safety shim

**Path:** `TRION/src/trion-runtime/src/kos_shim/mod.rs`

**Purpose:** Observe upstream KOS services and enforce safety/FDIR without modifying upstream code.

**Design:**
```rust
//! Safety shim between Trion runtime and upstream KOS services.
//!
//! The shim subscribes to KOS gRPC health/telemetry streams and can:
//! - issue safe-mode requests
//! - gate actuator commands through the command authority layer
//! - publish Trion health reports onto the Trion HealthBus

use crate::command::safe_mode::ModeController;
use crate::events::EventLog;
use crate::fdir::watchdog::HeartbeatPolicy;
use crate::health::{HealthBus, HealthLevel};
use crate::time::RuntimeClock;
use std::time::Duration;

pub struct KosSafetyShim {
    modes: ModeController,
    health: HealthBus,
    events: EventLog,
    clock: RuntimeClock,
}

impl KosSafetyShim {
    pub fn new(
        modes: ModeController,
        health: HealthBus,
        events: EventLog,
        clock: RuntimeClock,
    ) -> Self {
        Self { modes, health, events, clock }
    }

    /// Register a KOS service for heartbeat supervision.
    pub fn supervise_service(&self, name: &str, policy: HeartbeatPolicy) {
        // Returns a HeartbeatHandle that the shim beats on every successful gRPC ping.
    }

    /// Called when an actuator command is about to be sent to KOS.
    /// Returns Ok(cmds) if allowed, or Err(reason) if safe-mode / limits block it.
    pub fn validate_actuator_command(
        &self,
        commands: &[ActuatorCommand],
    ) -> eyre::Result<Vec<ActuatorCommand>> {
        if self.modes.is_safe() {
            return Err(eyre::eyre!("actuator commands rejected: safe-mode active"));
        }
        // Additional limit checks go here.
        Ok(commands.to_vec())
    }
}
```

**Integration notes:**
- The shim runs in the same process as `kos-kbot` (as a Trion wrapper binary) or as a sidecar that proxies gRPC calls.
- Preferred pattern: Trion provides a `trion-supervisor` binary that starts `kos-kbot` as a child process and proxies its gRPC socket.

### 4.2 Command authority and command classes

**Path:** `TRION/src/trion-runtime/src/command/authority.rs`

**Purpose:** Implement TR-P4-042 (immediate / queued / commit-window commands) and TR-P4-050 (authentication, roles, two-step arming).

**Design:**
```rust
//! Command authority layer.
//!
//! Commands are classified by latency tolerance and hazard:
//! - Immediate: E-stop, safe-mode, recovery. Always accepted if authenticated.
//! - Queued: motion sequences. Preconditions checked; executed in order.
//! - Commit-window: hazardous operations (motion in contact, power cycles).
//!   Operator must arm within a bounded window.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandClass {
    Immediate,
    Queued,
    CommitWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Observer,
    Operator,
    Supervisor,
}

pub struct CommandAuthority {
    operator: String,
    role: Role,
    armed_commands: HashMap<String, Instant>, // command token -> expiry
    window: Duration,
}

impl CommandAuthority {
    pub fn new(operator: String, role: Role, window: Duration) -> Self { /* ... */ }

    pub fn authorize(&self, class: CommandClass, token: &str) -> eyre::Result<()> {
        match class {
            CommandClass::Immediate => self.require_role(Role::Operator),
            CommandClass::Queued => self.require_role(Role::Operator),
            CommandClass::CommitWindow => self.require_armed(token),
        }
    }

    pub fn arm(&mut self, token: &str, hazard: &str) { /* audited; sets expiry */ }
}
```

**Traceability:** TR-P4-042, TR-P4-050.

### 4.3 Actuator command sanity gate

**Path:** `TRION/src/trion-runtime/src/command/gate.rs`

**Purpose:** Independent check that every command to KOS is within configured joint limits, velocity limits, and safe torque bounds.

**Design:**
```rust
use crate::config::ActuatorConfig;

pub struct CommandGate {
    config: HashMap<u32, ActuatorConfig>,
}

impl CommandGate {
    pub fn validate(
        &self,
        command: &ActuatorCommand,
    ) -> eyre::Result<ActuatorCommand> {
        let cfg = self.config.get(&command.actuator_id)
            .ok_or_else(|| eyre::eyre!("unknown actuator {}", command.actuator_id))?;

        let position = command.position
            .map(|p| p.clamp(cfg.limit_min, cfg.limit_max));
        let velocity = command.velocity
            .map(|v| v.clamp(-cfg.max_velocity, cfg.max_velocity));
        let torque = command.torque
            .map(|t| t.clamp(-cfg.max_torque, cfg.max_torque));

        Ok(ActuatorCommand { actuator_id: command.actuator_id, position, velocity, torque })
    }
}
```

**Integration:** Called by `KosSafetyShim::validate_actuator_command` before forwarding to KOS.

### 4.4 IMU health monitor

**Path:** `TRION/src/trion-runtime/src/fdir/imu_monitor.rs`

**Purpose:** Detect IMU stale/dropout/fault conditions and emit FDIR events.

**Design:**
```rust
//! IMU FDIR monitor (FDIR entry F-05).

use crate::events::{EventKind, EventLog};
use crate::health::{HealthBus, HealthLevel};
use crate::time::RuntimeClock;
use std::time::Duration;

pub struct ImuMonitor {
    timeout: Duration,
    last_good_ms: u64,
    events: EventLog,
    health: HealthBus,
    clock: RuntimeClock,
}

impl ImuMonitor {
    pub fn report_sample(&mut self) {
        self.last_good_ms = self.clock.now_ms();
        self.health.publish("imu", HealthLevel::Nominal, None);
    }

    pub fn tick(&mut self) {
        let silent = self.clock.now_ms().saturating_sub(self.last_good_ms);
        if silent > self.timeout.as_millis() as u64 {
            self.events.record(EventKind::FaultDetected, Some("imu"), "stale IMU data");
            self.health.publish("imu", HealthLevel::Critical, Some("stale"));
        }
    }
}
```

**Traceability:** FDIR F-05, TR-P4-013.

### 4.5 kbot-inference refactor design

The standalone `kbot-inference` crate should be retired as a direct hardware controller and replaced with a **Trion policy service**.

**New path:** `TRION/src/trion-policy-service/`

**Responsibilities:**
- Load ONNX policy.
- Subscribe to KOS actuator state + IMU via gRPC (no direct hardware).
- Run inference loop at 50 Hz.
- Forward commands through the Trion safety shim, not directly to actuators.
- Measure and report loop timing (WCET telemetry for TR-P4-060).

**Why a rewrite rather than patching `kbot-inference`:**
- Decouples policy execution from hardware drivers.
- Enables the safety gate to sit between NN output and actuator command.
- Allows a single config source and a single E-stop path.

**Migration plan:**
1. Implement `trion-policy-service` as a gRPC client to KOS.
2. Validate it in `TRION/sim/` against worksite W1 (dry-run mode).
3. Deprecate direct use of `kbot-inference` on hardware; document it as "legacy inference path."
4. Offer an upstream PR to K-Scale that adds a KOS-client mode to `kbot-inference`, preserving the crate for non-Trion users.

### 4.6 Configuration loader crate

**Path:** `TRION/src/trion-config/`

**Purpose:** Load `TRION/config/robot_manifest.yaml` and expose it to all Trion crates.

**Design:**
```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct RobotManifest {
    pub manifest_version: String,
    pub robot: RobotConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RobotConfig {
    pub serial: String,
    pub model: String,
    pub actuators: Vec<ActuatorConfig>,
    pub imus: Vec<ImuConfig>,
    pub power: Vec<PowerConfig>,
    pub video: Vec<VideoConfig>,
}

impl RobotManifest {
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> eyre::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let manifest: Self = serde_yaml::from_str(&text)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> eyre::Result<()> {
        // Verify unique actuator IDs, no duplicate bus assignments, etc.
        Ok(())
    }
}
```

**Usage:**
- `trion-runtime` reads it for limit checks and device paths.
- `trion-policy-service` reads it for observation/action mapping.
- A Python utility in `TRION/scripts/` reads it and emits `ACTUATOR_LIST` for `ksim-kbot/deploy`.

---

## 5. P2 — crate scaffolding

### 5.1 `TRION/src/trion-perception/`

**Purpose:** Vision-based localization, fixture recognition, worksite mapping (P2, non-safety-critical).

**Initial modules:**
- `localization.rs` — AprilTag / ArUco-based pose estimation against a worksite map.
- `fixture.rs` — detection of handles, latches, connectors, fasteners.
- `map.rs` — worksite map storage and update.

**Dependencies:** keep minimal; prefer OpenCV or a lightweight Rust CV crate. No dependency on `trion-runtime` to preserve safety/control separation (TR-P4-022).

### 5.2 `TRION/src/trion-skills/`

**Purpose:** Task skill library + autonomy-ladder wrapper (P1/P3).

**Initial modules:**
- `ladder.rs` — L0–L4 state machine and promotion criteria per [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md).
- `skills/inspect_fixture.rs` — T-01, L0–L1.
- `skills/actuate_handle.rs` — T-02, L0–L1.
- `skills/connector_mate.rs` — T-03, L3 target.
- `executor.rs` — multi-step plan executor with preconditions, abort conditions, human gates.

**Design rule:** every skill returns a `SkillOutcome` that is recorded in the event log. Skills never command actuators directly; they request commands through the command authority layer.

### 5.3 `TRION/src/mission-control/`

**Purpose:** Ground segment console (P5).

**Initial modules:**
- `telemetry.rs` — ingest Trion HealthBus / KOS telemetry.
- `dashboard.rs` — health matrix display.
- `plans.rs` — upload / review / execute task plans.
- `video.rs` — display KOS video stream.

**Implementation choice:** a Rust TUI (`ratatui`) for v0; web UI for v1.

---

## 6. P3 — manipulation and autonomous servicing

### 6.1 Worksuite training tasks in `TRION/sim/`

Add `TRION/sim/tasks/` with `ksim`-compatible task definitions:
- `inspect_task.py` — move head camera to fixture, capture image.
- `handle_task.py` — grasp and rotate handle.
- `connector_task.py` — insert plug into socket with force threshold.
- `torque_task.py` — apply bounded torque to a fastener.

Each task must include:
- reward shaping
- termination conditions
- fault-injection hooks (dropped IMU, actuator fault, delayed command)

### 6.2 End-effector abstraction

**Path:** `TRION/src/trion-skills/src/end_effector.rs`

**Purpose:** Define the tool/gripper interface early, even before hardware.

```rust
#[async_trait::async_trait]
pub trait EndEffector: Send + Sync {
    async fn open(&mut self) -> eyre::Result<()>;
    async fn close(&mut self, force_limit: f64) -> eyre::Result<()>;
    async fn read_force(&self) -> eyre::Result<[f64; 3]>;
}
```

**Initial implementation:** `FixedGripper` stub for sim; `KBotGripper` placeholder for future hardware.

---

## 7. Updates applied to existing plan documents

The following edits to [PROJECT_PLAN.md](PROJECT_PLAN.md) have been applied to keep the plan consistent with this recommendations doc:

1. **Companion artifacts list** now links to:
   - [NEXT_STEPS_AUDIT.md](NEXT_STEPS_AUDIT.md)
   - [UPSTREAM_KBOT_AUDIT.md](UPSTREAM_KBOT_AUDIT.md)
   - [RECOMMENDATIONS_AND_REWRITES.md](RECOMMENDATIONS_AND_REWRITES.md)

2. **Phase 0** updated to mark CI skeleton and W1 scene as done, and to include the robot configuration manifest with acceptance criteria.

3. **Phase 1** expanded to include:
   - Trion KOS safety shim
   - Trion policy service
   - Robot configuration manifest consumption
   - Command authority layer
   - Actuator command sanity gate
   - Updated exit criteria requiring safe-mode demonstration in sim and hardware

4. **Phase 2** updated to call out the new `trion-skills` and `trion-perception` crates explicitly.

### 7.1 Pending doc update: REQ_TEST_TRACEABILITY.md

Add rows for the new designs as planned tests:
- RT-06 — `command::authority` rejects unauthorized commit-window command.
- RT-07 — `command::gate` clips out-of-limit actuator command.
- RT-08 — `fdir::imu_monitor` detects stale IMU and emits event.
- T-SIM-12 — `trion-policy-service` runs worksite W1 dry-run.
- T-SIM-13 — E-stop stops motion within 100 ms in sim.
- T-SIM-14 — robot manifest validation rejects duplicate actuator IDs / bus conflicts at startup.

---

## 8. Non-goals (explicitly preserved)

- Do not modify `kos-kbot/`, `kbot-inference/`, or `ksim-kbot/` submodules directly.
- Do not attempt hardware space-rating (vacuum, thermal, radiation).
- Do not build a full web ground station in Phase 0 or Phase 1.
- Do not implement real vision processing in Phase 0.

---

## 9. Next concrete actions

1. **Create `TRION/config/robot_manifest.yaml`** with the K-Bot2 actuator/IMU/power/video schema.
2. **Create `TRION/sim/w1_panel/`** with scene, assets, and smoke test.
3. **Create `.github/workflows/trion-ci.yml`** and verify it runs on the current `trion-runtime`.
4. **Implement `trion-runtime/src/command/authority.rs` and `gate.rs`** as the first Phase 1 P4 modules.
5. **Implement `trion-runtime/src/fdir/imu_monitor.rs`** to close F-05.
6. **Scaffold `TRION/src/trion-policy-service/`** with a design doc and a minimal KOS gRPC client stub.

All of the above stay within `TRION/` and leave the upstream diff clean.
