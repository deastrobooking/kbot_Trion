# Trion Space Operations Engineer — Project Plan

**Mission:** Build a mission-critical space-operations mechanic/engineer on the K-Bot platform — a robot that can inspect, maintain, and repair infrastructure (habitats, ground stations, analog space facilities) under supervision or fully autonomously, engineered to the reliability standards of flight robotics.

**Strategy:** Copy the proven feature sets of the best systems in the business, implement them on the Trion/K-Bot stack (KOS + Rust runtime + ksim training), and hold the software to spaceflight-grade engineering practices from day one. Hardware space-rating (vacuum, thermal, radiation) is explicitly **out of scope** — we build the *operations engineer* and prove it in terrestrial/analog environments, the same way NASA and GITAI matured their systems on the ground and inside the ISS first.

---

## 1. Benchmark systems and what we copy from each

| System | Operator | Status | Feature set we copy |
| --- | --- | --- | --- |
| **Robonaut 2** | NASA JSC | Flew on ISS 2011–2018 | Humanoid form factor for human tools/workspaces; dexterous manipulation with force sensing; **shared control** (teleop with autonomous sub-behaviors); safety-rated operation near humans |
| **Astrobee** | NASA Ames | Operating on ISS | **Vision-based localization** (no external infrastructure); autonomous navigation, docking, perching; modular payload bay; **ground control + crew tablet interfaces**; fully open-source sim + flight software |
| **Dextre / Canadarm2 (SPDM)** | CSA/NASA | Operating on ISS | **Ground-commanded task sequences** with human "go/no-go" gates; ORU (Orbital Replacement Unit) change-out workflows; force-moment sensing to avoid damaging worksites; tool-changer paradigm |
| **GITAI S2 / S3 / Inchworm arms** | GITAI | TRL 7 (demoed outside ISS 2024); S3 servicing flight model complete (2026) | **Task-level autonomy** for ISAM (in-space servicing, assembly, manufacturing): panel assembly, connector mating, inspection, tool use; autonomy tiers from teleop → scripted → fully autonomous; designed-for-service arms with 1m/1.5m/10m variants |
| **Apptronik Apollo** | Apptronik + NASA | Factory work now, space is the roadmap | Modern humanoid architecture (NASA Valkyrie lineage); **learned manipulation policies** (DeepMind partnership); force-controlled safe actuation; mounted-torso *or* legged deployment modes |
| **Astrobee/cFS-class flight software discipline** | NASA | Standard practice | **FDIR** (fault detection, isolation, recovery); health telemetry on every subsystem; watchdogs and safe-mode; deterministic control loops; sim-first V&V |

The synthesis: **a humanoid mechanic (Robonaut/Apollo) with free-flyer-grade autonomy software (Astrobee), operated through Dextre-style supervised task sequences, executing GITAI-style servicing tasks, on an FDIR-hardened runtime.**

## 2. Capability pillars (the feature set)

### P1 — Supervised teleoperation with shared control
The entry point for every real space robot: a human drives, autonomy assists.
- Ground-station console: live video, robot state, joint-level and task-level command.
- Shared control: operator commands *intents* (grasp that handle), robot handles grasp closure, force limits, joint limits.
- Command latency tolerance: queue-and-execute task sequences with go/no-go gates (Dextre model), not just joystick streaming.
- Full session recording (K-Bot's `krec` telemetry) for replay and post-op analysis.

### P2 — Perception & localization (Astrobee model)
- Vision-based self-localization in a mapped worksite — no mocap, no external infrastructure.
- Worksite mapping: build and store maps of the facility the robot services.
- Object/fixture recognition: handles, connectors, panels, fasteners, tools, ORU-style modules.
- Inspection outputs: geotagged imagery, anomaly flags, before/after comparisons.

### P3 — Manipulation & tool use (Robonaut/GITAI model)
- Force-aware grasping and insertion (connector mating is *the* canonical ISAM task).
- Human-tool operation: torque tools, levers, latches — the humanoid advantage.
- Task skill library, each skill graded on the GITAI autonomy ladder: teleop → operator-triggered scripted → autonomous with human gate → fully autonomous.
- Trained policies from `ksim` where learning wins (locomotion, grasp closure), scripted/model-based control where determinism wins (torque sequences, safety-critical moves).

### P4 — Mission-critical runtime (the "flight software" bar)
This is what separates a demo from a *mission-critical* system:
- **FDIR on every subsystem**: detect (actuator overtemp/overcurrent, IMU dropout, comm loss, watchdog miss), isolate (which joint/board/service), recover (retry, degrade, safe-mode).
- **Safe-mode**: a verified minimal state — brakes/zero-torque posture, heartbeat telemetry, accepts only recovery commands.
- **Health telemetry bus**: every KOS service publishes rate-limited health packets; ground console shows a system health matrix (green/yellow/red per subsystem).
- **Watchdogs**: process manager supervises every service; missed heartbeat → automatic restart or safe-mode escalation.
- **Deterministic control loop budget**: the inference/control path (Rust) has a measured worst-case execution time and a frame-overrun policy.
- **Coding standard**: Rust with upstream K-Scale rules (no `unwrap`/`expect`, `eyre` errors, `tracing`) plus JPL "Power of Ten"-inspired rules: bounded loops, bounded memory after init, every return value checked, assertions on invariants.
- **Command authentication & two-step arming** for hazardous commands (motion in contact, power cycling).

### P5 — Mission operations layer (ground segment)
- Task plan format: declarative mission sequences (steps, preconditions, abort conditions, human gates) — the Astrobee "plan" concept.
- Mission control UI: plan upload, step-through execution, telemetry dashboards, video, manual override, e-stop.
- Ops products: automatic post-mission reports (timeline, anomalies, telemetry excerpts).
- Procedures-as-code: every maintenance task is a reviewed, versioned, simulated-before-flown artifact.

### P6 — Simulation-first V&V (how flight software earns trust)
- Digital twin of the robot *and the worksite* in MuJoCo (via `ksim`).
- Every task plan must pass N sim runs (including injected faults) before running on hardware.
- Fault-injection harness: kill a service, drop the IMU, spike a joint temperature mid-task — verify FDIR responds as specified.
- Regression suite: skills and FDIR behaviors re-verified in sim on every change (CI).

## 3. Architecture mapping to the Trion stack

```
┌──────────────────────────────────────────────────────────────┐
│  TRION/src/mission-control   (P5)  — ground console, plans   │
│  web UI + gRPC client to KOS; plan executor; telemetry dash  │
└──────────────────────────────┬───────────────────────────────┘
                        gRPC (KOS services)
┌──────────────────────────────▼───────────────────────────────┐
│  TRION/src/trion-runtime     (P4)  — Rust, on-robot          │
│  FDIR supervisor · health bus · safe-mode · command auth     │
│  wraps/extends kos-kbot services, supervises via             │
│  process-manager pattern                                     │
├──────────────────────────────────────────────────────────────┤
│  TRION/src/trion-skills      (P1/P3) — Rust + ONNX policies  │
│  skill library: grasp, insert, torque, inspect;              │
│  autonomy-ladder wrapper (teleop→scripted→gated→autonomous)  │
├──────────────────────────────────────────────────────────────┤
│  TRION/src/trion-perception  (P2) — localization, fixture    │
│  recognition, worksite mapping                               │
└──────────────────────────────┬───────────────────────────────┘
                     upstream, unmodified
┌──────────────────────────────▼───────────────────────────────┐
│  kos-kbot (actuators/IMU/video) · kbot-inference (ONNX loop) │
│  ksim-kbot + TRION/sim (training + digital twin + fault inj) │
└──────────────────────────────────────────────────────────────┘
```

Rules of engagement (per [CONTRIBUTING.md](CONTRIBUTING.md)): upstream submodules stay unmodified; Trion consumes KOS gRPC services and published crates. Genuine platform gaps become upstream PRs.

## 4. Phased roadmap

### Phase 0 — Foundations (now)
- [ ] Stand up dev environment: cross-compile toolchain, sim running locally, CI skeleton.
- [ ] `TRION/src/trion-runtime` crate: health-telemetry bus + heartbeat watchdog for one KOS service (walking skeleton of P4).
- [ ] Digital twin: K-Bot in MuJoCo with a first mock worksite (a panel with a handle and a connector).
- [ ] Requirements doc: written definitions of safe-mode, FDIR matrix v0 (top 10 faults), command-authority levels.
- **Exit criteria:** robot (sim) streams health telemetry; killing a service triggers detection + logged recovery within a bounded time.

### Phase 1 — Teleop mechanic (Robonaut baseline)
- [ ] Mission-control console v0: video + state + joint/Cartesian teleop + e-stop over gRPC.
- [ ] Session recording/replay (`krec`) wired into every run.
- [ ] Shared-control grasp: operator designates target, robot executes force-limited grasp.
- [ ] First two skills on real hardware: *inspect fixture* (camera survey) and *actuate handle*.
- **Exit criteria:** a remote operator completes an inspect-and-actuate task end-to-end with recorded telemetry.

### Phase 2 — Supervised autonomy (Dextre/Astrobee level)
- [ ] Task-plan format + executor with preconditions, abort conditions, human go/no-go gates.
- [ ] Vision localization against the mapped worksite; fixture recognition for the skill library.
- [ ] Sim-gating pipeline: plans must pass fault-injected sim runs before hardware execution.
- [ ] FDIR v1: full detect/isolate/recover matrix, safe-mode entry demonstrated from every fault class.
- **Exit criteria:** operator uploads a multi-step maintenance plan; robot executes with one human gate; injected fault mid-task → clean safe-mode → resume.

### Phase 3 — Autonomous servicing tasks (GITAI level)
- [ ] Canonical ISAM skills, autonomous with gates: connector mate/demate, panel fastener sequence, tool pickup + torque operation, module (ORU-style) swap.
- [ ] Skill training in `ksim` where applicable; ONNX export into the Rust inference path.
- [ ] Anomaly-driven behavior: inspection detects an out-of-spec condition → proposes a repair plan for approval.
- **Exit criteria:** GITAI-style demo — robot autonomously completes an assembly/servicing sequence on the mock worksite, human only approving gates.

### Phase 4 — Mission-critical certification posture
- [ ] Requirements → test traceability (every P4 requirement has a verifying test).
- [ ] WCET measurement of the control path; frame-overrun policy verified.
- [ ] 100-run reliability campaign per flagship skill in sim + N-run hardware campaign; publish MTBF-style numbers.
- [ ] Ops manual + procedures library; post-mission report generator.
- **Exit criteria:** a documented, evidence-backed reliability case — the difference between "it worked in the video" and "mission critical."

## 5. Risks

| Risk | Mitigation |
| --- | --- |
| Upstream K-Bot alpha churn (breaking changes) | Submodule pinning; upstream sync is a deliberate, tested event (see CONTRIBUTING.md) |
| Sim-to-real gap on contact-rich tasks (insertion) | Force-feedback-centric skills; scripted/model-based fallback per skill; hardware iteration loops early (Phase 1) |
| Scope creep toward actual space-rating | Explicit non-goal; analog worksite defines "done" |
| Single-robot hardware availability | Sim-first pipeline means 90% of development never touches hardware |
| GPL/MIT license mixing | Submodules are MIT; keep Trion runtime depending only on MIT crates (see Developer Guide licensing note) |

## 6. Immediate next actions

1. Scaffold `TRION/src/trion-runtime` (Rust workspace) with the health-telemetry walking skeleton.
2. Scaffold `TRION/sim/` with the digital-twin worksite scene.
3. Write `TRION/docs/FDIR_MATRIX.md` v0 (top-10 fault table: detection signal, isolation, recovery, safe-mode trigger).
4. Write `TRION/docs/REQUIREMENTS.md` v0 for P4 (safe-mode, watchdog, telemetry bus definitions).

---

*Benchmark sources: NASA Robonaut 2 (ISS 2011–2018), NASA Astrobee open-source flight software, CSA Dextre/SPDM ops model, GITAI S2 ISS demo (TRL 7, 2024) and S3 servicing flight model (2026), Apptronik Apollo + NASA/DeepMind partnerships. See PR/commit description for links.*
