# Trion Space Operations Engineer — Project Plan

**Mission:** Build a mission-critical space-operations mechanic/engineer on the K-Bot platform — a robot that can inspect, maintain, and repair infrastructure (habitats, ground stations, analog space facilities) under supervision or fully autonomously, engineered to the reliability standards of flight robotics.

**Strategy:** Copy the proven feature sets of the best systems in the business, implement them on the Trion/K-Bot stack (KOS + Rust runtime + ksim training), and hold the software to spaceflight-grade engineering practices from day one. Hardware space-rating (vacuum, thermal, radiation) is explicitly **out of scope** — we build the *operations engineer* and prove it in terrestrial/analog environments, the same way NASA and GITAI matured their systems on the ground and inside the ISS first.

**Companion artifacts** (this plan is the index; the details live in):

- [REQUIREMENTS.md](REQUIREMENTS.md) — numbered P4 requirements with verification methods
- [PHA.md](PHA.md) — preliminary hazard analysis (system boundaries, RAC-ranked hazard worksheet)
- [SAFETY_CASE.md](SAFETY_CASE.md) — safety goals, argument structure, claims-to-evidence mapping
- [FDIR_MATRIX.md](FDIR_MATRIX.md) — fault detection/isolation/recovery matrix
- [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md) — requirements-to-test matrix with coverage targets
- [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md) — formal L0–L5 autonomy ladder, skill spec template, promotion criteria
- [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md) — canonical worksites and task set

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

Specific operational lessons adopted:

- **Robonaut 2:** safety first, control second — safety logic is verified before capabilities are added, kept architecturally separate from non-critical services, and lives local to the robot. Predictable, crew-benign behavior is a feature, not a nicety.
- **Dextre:** ORU change-out workflows with force-moment sensing discipline; ground-commanded sequences with go/no-go gates rather than joystick streaming.
- **Astrobee:** plan-based execution, infrastructure-free vision localization, and rich telemetry into a real ground segment.
- **GITAI:** the autonomy ladder and the ISAM task set are the canonical Phase 3 target; skills earn promotion up the ladder with recorded evidence.

## 2. Capability pillars (the feature set)

### P1 — Supervised teleoperation with shared control
The entry point for every real space robot: a human drives, autonomy assists.
- Ground-station console: live video, robot state, joint-level and task-level command.
- Shared control: operator commands *intents* (grasp that handle), robot handles grasp closure, force limits, joint limits.
- Command latency tolerance: queue-and-execute task sequences with go/no-go gates (Dextre model), not just joystick streaming. Latency budgets, degraded comms modes, and delay-tolerant command classes are formal requirements (TR-P4-040…043).
- Full session recording (K-Bot's `krec` telemetry) for replay and post-op analysis.

### P2 — Perception & localization (Astrobee model)
- Vision-based self-localization in a mapped worksite — no mocap, no external infrastructure.
- Worksite mapping: build and store maps of the facility the robot services.
- Object/fixture recognition: handles, connectors, panels, fasteners, tools, ORU-style modules.
- Inspection outputs: geotagged imagery, anomaly flags, before/after comparisons.

### P3 — Manipulation & tool use (Robonaut/GITAI model)
- Force-aware grasping and insertion (connector mating is *the* canonical ISAM task).
- Human-tool operation: torque tools, levers, latches — the humanoid advantage.
- Task skill library, each skill graded on the formal autonomy ladder (L0 teleop → L4 fully autonomous, see [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md)) with evidence-based promotion criteria.
- End-effector interface standard defined early (Dextre/GITAI tool-changer paradigm) so tool changers can be added later without redesign, even while we start with a fixed gripper.
- Trained policies from `ksim` where learning wins (locomotion, grasp closure), scripted/model-based control where determinism wins (torque sequences, safety-critical moves).

### P4 — Mission-critical runtime (the "flight software" bar)
This is what separates a demo from a *mission-critical* system. Requirements are numbered in [REQUIREMENTS.md](REQUIREMENTS.md); highlights:
- **Preliminary hazard analysis (PHA) and safety case** maintained from Phase 0; every P4 requirement traces to hazards, FDIR entries, and tests ([PHA.md](PHA.md)).
- **Safety/control separation** (Robonaut 2 lesson): safety logic — limits, e-stop, safe-mode — cannot depend on non-critical services (high-level perception, network); it runs local to the robot.
- **FDIR on every subsystem**: detect (actuator overtemp/overcurrent, IMU dropout, comm loss, watchdog miss), isolate (which joint/board/service), recover (retry, degrade, safe-mode). See [FDIR_MATRIX.md](FDIR_MATRIX.md).
- **Safe-mode**: a verified minimal state — brakes/zero-torque posture, heartbeat telemetry, accepts only authenticated recovery commands.
- **Health telemetry bus**: every KOS service publishes rate-limited health packets; ground console shows a system health matrix (green/yellow/red per subsystem).
- **Watchdogs**: process manager supervises every service; missed heartbeat → automatic restart or safe-mode escalation.
- **Comms & time model**: defined latency/jitter budgets per link, degraded modes (telemetry-only, store-and-forward), a single traceable time base (ground NTP/PTP, robot monotonic clock, bounded skew) stamping all telemetry, events, and `krec` logs.
- **Deterministic control loop budget**: the inference/control path (Rust) has a measured worst-case execution time and a frame-overrun policy.
- **Coding standard**: Rust with upstream K-Scale rules (no `unwrap`/`expect`, `eyre` errors, `tracing`) plus JPL "Power of Ten"-inspired rules: bounded loops, bounded memory after init, every return value checked, assertions on invariants.
- **Command authority & security model**: authenticated commands, role-based command classes, two-step arming for hazardous commands (motion in contact, power cycling), tamper-evident logs.

### P5 — Mission operations layer (ground segment)
- Task plan format: declarative mission sequences (steps, preconditions, abort conditions, human gates) — the Astrobee "plan" concept.
- Mission control UI: plan upload, step-through execution, telemetry dashboards, video, manual override, e-stop. A human-factors review covers clarity of state, e-stop placement, and feedback during shared-control actions (Robonaut 2 lesson: predictability earns operator trust).
- **Defined data model**: telemetry schema (timestamp, subsystem, health, key metrics), structured event log (mode changes, FDIR triggers, human gates, skill start/stop, errors, recoveries), and a post-mission report format (summary, timeline, anomalies + FDIR actions, telemetry highlights, lessons learned) generated automatically from logs.
- All mission data stored for offline analysis and model retraining.
- Procedures-as-code: every maintenance task is a reviewed, versioned, simulated-before-flown artifact.

### P6 — Simulation-first V&V (how flight software earns trust)
- Digital twin of the robot *and the worksite* in MuJoCo (via `ksim`).
- **Requirements-to-test traceability**: every requirement has one or more verifying tests (sim and/or hardware), tracked in a traceability matrix.
- **Coverage targets**: 100% of P4 safety requirements verified in sim; ≥90% of canonical tasks passing fault-injected sim runs before hardware; 100% of FDIR entries exercised in sim per release.
- Fault-injection harness: kill a service, drop the IMU, spike a joint temperature mid-task — verify FDIR responds as specified.
- **Campaign metrics**: per skill/task — success rate, time to complete, faults encountered, FDIR actions taken; reliability computed as task success probability and mean cycles between safety interventions.
- CI: every PR runs unit tests + a sim scenario subset including at least one fault injection; merge blocked if safety-critical tests fail or regress.

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
│  SAFETY-CRITICAL: no dependency on perception/network        │
│  wraps/extends kos-kbot services, supervises via             │
│  process-manager pattern                                     │
├──────────────────────────────────────────────────────────────┤
│  TRION/src/trion-skills      (P1/P3) — Rust + ONNX policies  │
│  skill library: grasp, insert, torque, inspect;              │
│  autonomy-ladder wrapper (L0→L4, evidence-gated promotion)   │
├──────────────────────────────────────────────────────────────┤
│  TRION/src/trion-perception  (P2) — localization, fixture    │
│  recognition, worksite mapping (non-safety-critical tier)    │
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
- [x] `TRION/src/trion-runtime` crate: health-telemetry bus + heartbeat watchdog + FDIR escalation + safe-mode (walking skeleton of P4; requirement-traced tests + demo).
- [x] **Preliminary Hazard Analysis (PHA) v0 and safety case skeleton** ([PHA.md](PHA.md)) feeding the FDIR matrix.
- [x] **Comms & time-sync model**: latency budgets, single time base, delay-tolerant command classes (in [REQUIREMENTS.md](REQUIREMENTS.md)).
- [x] **Canonical worksites and task set** defined ([WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md)) — these drive skill design and sim scenarios from day one.
- [ ] Digital twin: K-Bot in MuJoCo with worksite W1 (panel with handle, latch, connector).
- [x] Requirements doc v0: safe-mode, FDIR matrix v0, command-authority levels ([REQUIREMENTS.md](REQUIREMENTS.md), [FDIR_MATRIX.md](FDIR_MATRIX.md)).
- **Exit criteria:** robot (sim) streams health telemetry; killing a service triggers detection + logged recovery within a bounded time; PHA v0 reviewed.

### Phase 1 — Teleop mechanic (Robonaut baseline)
- [ ] Mission-control console v0: video + state + joint/Cartesian teleop + e-stop over gRPC (human-factors pass on the UI).
- [ ] Session recording/replay (`krec`) wired into every run; telemetry + event schemas implemented.
- [ ] Shared-control grasp: operator designates target, robot executes force-limited grasp.
- [ ] First two skills on real hardware at L0–L1: *inspect fixture* (T-01) and *actuate handle* (T-02).
- **Exit criteria:** a remote operator completes an inspect-and-actuate task end-to-end with recorded telemetry and an auto-generated post-mission report.

### Phase 2 — Supervised autonomy (Dextre/Astrobee level)
- [ ] Task-plan format + executor with preconditions, abort conditions, human go/no-go gates, and the three command classes (immediate / queued / commit-window).
- [ ] Vision localization against the mapped worksite; fixture recognition for the skill library.
- [ ] Sim-gating pipeline: plans must pass fault-injected sim runs before hardware execution.
- [ ] FDIR v1: full detect/isolate/recover matrix, safe-mode entry demonstrated from every fault class.
- [ ] **Requirements-to-test traceability matrix v1**; initial coverage targets measured and reported.
- **Exit criteria:** operator uploads a multi-step maintenance plan; robot executes with one human gate; injected fault mid-task → clean safe-mode → resume.

### Phase 3 — Autonomous servicing tasks (GITAI level)
- [ ] Canonical ISAM skills at L3 (gated autonomy): connector mate/demate (T-03), fastener torque (T-04), tool pickup + use (T-06), ORU-style module swap (T-05).
- [ ] Skill training in `ksim` where applicable; ONNX export into the Rust inference path.
- [ ] Anomaly-driven behavior (T-07): inspection detects an out-of-spec condition → proposes a repair plan for approval.
- [ ] Skills promoted up the autonomy ladder only on documented promotion criteria (sim/hardware run counts, zero safety violations, FDIR coverage).
- **Exit criteria:** GITAI-style demo — robot autonomously completes an assembly/servicing sequence on the mock worksite, human only approving gates.

### Phase 4 — Mission-critical certification posture
- [ ] Requirements → test traceability complete (every P4 requirement has a verifying test).
- [ ] **Safety case v1** with evidence from sim and hardware campaigns; **PHA closed out** for all identified hazards.
- [ ] WCET measurement of the control path; frame-overrun policy verified.
- [ ] 100-run reliability campaign per flagship skill in sim + N-run hardware campaign; publish success-probability and safety-intervention metrics.
- [ ] Ops manual + procedures library; post-mission report generator hardened.
- **Exit criteria:** a documented, evidence-backed reliability case — the difference between "it worked in the video" and "mission critical."

## 5. Risks

| Risk | Mitigation |
| --- | --- |
| Upstream K-Bot alpha churn (breaking changes) | Submodule pinning; upstream sync is a deliberate, tested event (see CONTRIBUTING.md) |
| Sim-to-real gap on contact-rich tasks (insertion) | Force-feedback-centric skills; scripted/model-based fallback per skill; hardware iteration loops early (Phase 1) |
| Scope creep toward actual space-rating | Explicit non-goal; analog worksite defines "done" |
| Single-robot hardware availability | Sim-first pipeline means 90% of development never touches hardware |
| GPL/MIT license mixing | Submodules are MIT; keep Trion runtime depending only on MIT crates (see Developer Guide licensing note) |
| Safety logic entangled with non-critical code over time | TR-P4-022 design constraint + architecture review at each phase gate |

## 6. Immediate next actions

1. ~~Scaffold `TRION/src/trion-runtime` (Rust workspace) with the health-telemetry + watchdog + safe-mode walking skeleton.~~ **Done** — see [src/trion-runtime/](../src/trion-runtime/).
2. ~~Write [REQUIREMENTS.md](REQUIREMENTS.md) v0, [FDIR_MATRIX.md](FDIR_MATRIX.md) v0, [PHA.md](PHA.md) v0.~~ **Done.**
3. ~~Write [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md) and [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md).~~ **Done.**
4. Scaffold `TRION/sim/` with the digital-twin worksite scene (W1).
5. CI skeleton: build + test + clippy for `TRION/src` on every PR (TR-P4-070 gate).

---

*Benchmark sources: NASA Robonaut 2 (ISS 2011–2018, lessons-learned NTRS 20130012872), NASA Astrobee open-source flight software, CSA Dextre/SPDM ops model, GITAI S2 ISS demo (TRL 7, 2024) and S3 servicing flight model (2026), Apptronik Apollo + NASA/DeepMind partnerships.*
