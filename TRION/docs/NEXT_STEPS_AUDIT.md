# Trion Project Review & Next-Steps Audit

**Date:** 2026-08-02  
**Scope:** TRION layer only (`TRION/`) against [PROJECT_PLAN.md](PROJECT_PLAN.md) Phase 0 exit criteria.  
**Auditor:** GitHub Copilot

---

## Executive summary

The project has a strong Phase 0 documentation foundation and a coherent P4 runtime skeleton, but the *executable* parts of Phase 0 are not yet closed. The runtime crate has been reorganized into a clean tiered module layout (time / telemetry / FDIR / command), yet local build verification is currently blocked by a Rustup sandbox permission issue. The two largest remaining Phase 0 deliverables — the MuJoCo digital-twin worksite (`TRION/sim/`) and a TRION-specific CI pipeline — do not exist yet. Until those are in place, the Phase 0 exit criteria cannot be demonstrated.

| Pillar | Status | Confidence |
|--------|--------|------------|
| P4 runtime skeleton | 🟡 Implemented, build not locally verified | Medium |
| P4 docs (PHA, FDIR, requirements, traceability, autonomy, worksites, safety case) | 🟢 Complete v0 | High |
| P1/P2/P3 crates / skills / perception / ground segment | 🔴 Not started | High |
| P5 mission-control / data model / procedures | 🔴 Not started | High |
| P6 simulation-first V&V (digital twin W1 + fault-injection harness) | 🔴 Not started | High |
| CI / build + test + clippy gate | 🔴 Not started | High |

---

## Phase 0 scorecard

| # | Deliverable | Plan reference | Status | Evidence / blocker |
|---|-------------|----------------|--------|--------------------|
| 0.1 | Cross-compile toolchain + local sim | Phase 0 bullet 1 | 🟡 Partial | Toolchain docs in [DEVELOPER_GUIDE.md](../DEVELOPER_GUIDE.md); no local sim scene yet |
| 0.2 | `trion-runtime` walking skeleton (health + watchdog + FDIR + safe-mode) | Phase 0 bullet 2 | 🟡 Implemented | [src/trion-runtime/src/lib.rs](../src/trion-runtime/src/lib.rs#L1) and module tiers: [time/](../src/trion-runtime/src/time/mod.rs), [telemetry/](../src/trion-runtime/src/telemetry/mod.rs), [fdir/](../src/trion-runtime/src/fdir/mod.rs), [command/](../src/trion-runtime/src/command/mod.rs) |
| 0.3 | PHA v0 + safety case skeleton | Phase 0 bullet 3 | 🟢 Done | [PHA.md](PHA.md), [SAFETY_CASE.md](SAFETY_CASE.md) |
| 0.4 | Comms & time-sync model | Phase 0 bullet 4 | 🟢 Requirements | [REQUIREMENTS.md](REQUIREMENTS.md) TR-P4-040…043; runtime clock in [time/mod.rs](../src/trion-runtime/src/time/mod.rs) |
| 0.5 | Canonical worksites + task set | Phase 0 bullet 5 | 🟢 Done | [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md) |
| 0.6 | Digital twin: K-Bot in MuJoCo with worksite W1 | Phase 0 bullet 6 | 🔴 Missing | No `TRION/sim/` directory |
| 0.7 | Requirements v0 + FDIR matrix v0 | Phase 0 bullet 7 | 🟢 Done | [REQUIREMENTS.md](REQUIREMENTS.md), [FDIR_MATRIX.md](FDIR_MATRIX.md) |
| 0.8 | CI skeleton: build + test + clippy for `TRION/src` | Phase 0 bullet 8 / TR-P4-070 | 🔴 Missing | No `.github/workflows/` under `TRION/`; only upstream workflows in `kos-kbot/` and `ksim-kbot/` |

**Phase 0 exit criteria:**
> Robot (sim) streams health telemetry; killing a service triggers detection + logged recovery within a bounded time; PHA v0 reviewed.

- PHA v0 is written but there is no evidence of review recorded.
- The runtime demo exists ([walking_skeleton.rs](../src/trion-runtime/src/bin/walking_skeleton.rs)), but it cannot be exercised without a working build.
- There is no simulation scene, so the "robot (sim)" portion is not satisfied.

**Verdict:** Phase 0 is **not closed**.

---

## Architecture audit

### What exists

The runtime is the only implemented crate. It is organized into four tiers that match the P4 safety/control-separation intent:

1. **[time/](../src/trion-runtime/src/time/mod.rs)** — monotonic `RuntimeClock` (TR-P4-002).
2. **[telemetry/](../src/trion-runtime/src/telemetry/mod.rs)** — bounded `HealthBus` and `EventLog` (TR-P4-001, TR-P4-003, TR-P4-030).
3. **[fdir/](../src/trion-runtime/src/fdir/mod.rs)** — heartbeat `Watchdog` and `Supervisor` escalation (TR-P4-010…012, FDIR F-01).
4. **[command/](../src/trion-runtime/src/command/mod.rs)** — `ModeController` / safe-mode state machine (TR-P4-020, TR-P4-021).

Public API is re-exported cleanly from [lib.rs](../src/trion-runtime/src/lib.rs#L18). The unit and integration tests in [tests/runtime.rs](../src/trion-runtime/tests/runtime.rs) name the requirements they verify, and the traceability matrix in [REQ_TEST_TRACEABILITY.md](REQ_TEST_TRACEABILITY.md) is up to date.

### What is missing vs. the plan

Per the architecture diagram in [PROJECT_PLAN.md](PROJECT_PLAN.md#3-architecture-mapping-to-the-trion-stack), the following crates/directories do not exist:

| Planned path | Pillar | Purpose | Status |
|--------------|--------|---------|--------|
| `TRION/src/trion-skills` | P1/P3 | Skill library + autonomy-ladder wrapper | 🔴 Not created |
| `TRION/src/trion-perception` | P2 | Localization, fixture recognition, mapping | 🔴 Not created |
| `TRION/src/mission-control` | P5 | Ground console, plan executor, telemetry dash | 🔴 Not created |
| `TRION/sim/` | P6 | MuJoCo digital twin + worksite W1 + fault-injection harness | 🔴 Not created |

These omissions are expected for Phase 0, but they should be explicitly scheduled before claiming Phase 1 entry.

---

## Build / test status

A `cargo test` run from [TRION/src/](../src/Cargo.toml) was attempted. The first invocation compiled against a stale `lib.rs` that still referenced the old flat modules (`clock`, `events`, `health`, `safe_mode`, `supervisor`, `watchdog`) and failed with `E0583 file not found`. The current `lib.rs` has since been updated to the tiered module structure (`time`, `telemetry`, `fdir`, `command`). Subsequent `cargo` invocations are blocked in this environment by:

```text
error: could not write settings file: '/Users/home/.rustup/settings.toml': Operation not permitted (os error 1)
```

This is a sandbox/filesystem restriction, not a code defect, but it means **the test suite has not been verified passing in this session**.

### Recommended verification steps

1. On a machine with an unrestricted Rust toolchain, run:
   ```bash
   cd TRION/src
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   cargo run --bin walking_skeleton
   ```
2. Add a CI workflow that runs the same commands so verification is no longer host-dependent (see next actions).

---

## Risks & blockers

| Risk | Current state | Mitigation |
|------|---------------|------------|
| Local build/test cannot be verified | Rustup sandbox write restriction | Move verification to CI; document local toolchain requirements |
| Phase 0 could be declared done without a sim | No `TRION/sim/` exists | Add the worksite W1 scene as the next deliverable |
| Safety logic drift | Only F-01 (watchdog) is implemented; [FDIR_MATRIX.md](FDIR_MATRIX.md) lists F-02…F-08 | Keep FDIR matrix as the backlog; implement one entry per sprint |
| Upstream submodule churn | Submodules are pinned and unmodified | Continue honoring [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENT_GUIDE.md](../AGENT_GUIDE.md) |
| Scope creep toward space-rating | Explicitly out of scope in plan | Reiterate "analog worksite defines done" at every phase gate |

---

## Prioritized next actions

### P0 — close Phase 0

1. **Verify the build**
   - Run `cargo test`, `cargo clippy`, and `cargo fmt --check` in a clean environment (or CI).
   - If the stale-lib issue recurs, `cargo clean` and rerun.
   - Acceptance: all tests pass, clippy clean, formatter clean.

2. **Add TRION CI**
   - Create [`.github/workflows/trion-ci.yml`](../.github/workflows/trion-ci.yml) that:
     - Checks out the repo with submodules.
     - Runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` inside `TRION/src`.
     - Optionally runs the `walking_skeleton` demo and captures its exit code.
   - This directly satisfies TR-P4-070 and removes the local-toolchain dependency.

3. **Create `TRION/sim/` with worksite W1**
   - Add a MuJoCo scene file for the K-Bot + panel (handle, latch, connector) described in [WORKSITES_AND_TASKS.md](WORKSITES_AND_TASKS.md).
   - Reuse `ksim-kbot` conventions and assets; keep the scene under `TRION/sim/` per [AGENT_GUIDE.md](../AGENT_GUIDE.md).
   - Acceptance: a script can load the scene and step physics.

4. **Record PHA v0 review**
   - Add a review note/log entry to [PHA.md](PHA.md) or a `reviews/` folder (reviewer, date, outcome).
   - Acceptance: PHA v0 is marked reviewed with at least one sign-off.

### P1 — start Phase 1 work

5. **Mission-control crate skeleton (`TRION/src/mission-control`)**
   - Rust crate with a gRPC client to KOS, telemetry ingestion from `HealthBus`, and a minimal TUI/web stub.
   - Acceptance: can subscribe to a mocked health bus and display a health matrix.

6. **`krec` session recording integration**
   - Wire `krec` telemetry recording into the runtime event/health streams.
   - Acceptance: a demo run produces a replayable `krec` log.

7. **First two skills at L0–L1**
   - Begin `TRION/src/trion-skills` with *inspect fixture* (T-01) and *actuate handle* (T-02) as scripted, human-gated skills.
   - Acceptance: each skill has a skill spec per [AUTONOMY_LEVELS.md](AUTONOMY_LEVELS.md) and a sim test.

### P2 — readiness for supervised autonomy

8. **Implement remaining FDIR monitors**
   - Translate F-02…F-08 from [FDIR_MATRIX.md](FDIR_MATRIX.md) into runtime monitors (actuator thermal/current, CAN loss, IMU staleness, frame-overrun, comms loss, power).
   - Acceptance: each entry has a sim fault-injection test.

9. **Command authority layer**
   - Add the three command classes (immediate / queued / commit-window), authentication stub, and two-step arming for hazardous commands (TR-P4-042, TR-P4-050).
   - Acceptance: sim tests reject unauthorized/unarmed commands.

10. **Digital-twin fault-injection harness**
    - Build on the W1 sim to inject faults mid-task and verify FDIR response.
    - Acceptance: at least one canonical task survives a fault without safety violation.

---

## Suggested short-term sprint goal

> **"Phase 0 is green: TRION builds and tests in CI, and the K-Bot can stand in the W1 worksite in MuJoCo."**

This keeps scope tight, produces demonstrable evidence, and unblocks Phase 1 with a known-good foundation.

---

## Addendum — resolution log (2026-08-02, same day)

| Audit action | Status | Evidence |
|--------------|--------|----------|
| P0-1 Verify the build | ✅ Closed | Verified in an unrestricted environment: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (5/5 pass), and `cargo run --bin walking_skeleton` (exit 0) all green. The earlier failure was the auditing session's Rustup sandbox restriction, not a code defect. |
| P0-2 Add TRION CI | ✅ Closed | [.github/workflows/trion-ci.yml](../../.github/workflows/trion-ci.yml): fmt + clippy (deny warnings) + tests + demo, path-filtered to `TRION/**`. Root location is a documented exception to the TRION-only rule (GitHub Actions requirement). |
| P0-3 Create `TRION/sim/` with worksite W1 | ✅ Closed (worksite) | [sim/worksites/w1_panel.xml](../sim/worksites/w1_panel.xml) + [sim/load_scene.py](../sim/load_scene.py); acceptance met — scene loads and steps 2000 steps stably (MuJoCo 3.1.6). **Open remainder:** composing the K-Bot model into the scene (needs `kscale-assets` from ksim-kbot's nested submodule). |
| P0-4 Record PHA v0 review | 🔴 Open | Requires a human reviewer sign-off — not something automation should self-certify. |

Phase 0 status after addendum: all deliverables in place except **K-Bot model composition into W1** and **PHA review sign-off**.
