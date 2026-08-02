# Trion Project Review & Next-Steps Audit

**Date:** 2026-08-02  
**Scope:** TRION layer only (`TRION/`) against [PROJECT_PLAN.md](PROJECT_PLAN.md) Phase 0 exit criteria.

## Executive summary

Trion has a green native Rust foundation, a requirement-traced P4 runtime, a composed pinned K-Bot + W1 model, and CI gates for Rust plus MuJoCo. Phase 0 is not closed because the transport-agnostic safety path is not yet connected to MuJoCo or real KOS, and PHA v0 still needs human review.

The next milestone is one vertical safety demonstration: **K-Bot in W1 receives a command through the Trion safety gate; an injected fault causes a measured safe actuator response and a structured event record.**

## Verified status

| Deliverable | Status | Evidence / open remainder |
| --- | --- | --- |
| Native Rust gates | ✅ | `cargo fmt --check`, Clippy with warnings denied, and all tests pass |
| P4 walking skeleton | ✅ Logic skeleton | Health, watchdog, restart escalation, mode transition, explicit recovery, JSONL events |
| Telemetry bounds | ✅ Unit verified | Zero capacity rejected; unobserved and receiver-overflow drops counted |
| Watchdog bound | ✅ Unit verified | Sampling is constrained to the heartbeat interval; deterministic elapsed-time test covers TR-P4-011 |
| PHA, requirements, FDIR, traceability | ✅ v0 artifacts | PHA human review/sign-off remains open |
| W1 MuJoCo worksite | ✅ Composed | Standalone fixture runs 2,000 steps; pinned K-Bot + W1 loads as one 20-actuator model and runs a 100-step smoke |
| TRION CI | ✅ Skeleton | Rust gates, runtime demo, and headless W1 smoke test |
| Command safety gate | 🟡 Core verified | Manifest validation, limit clamping, unknown/non-finite rejection, and safe-mode rejection are unit tested; KOS integration remains open |
| Command authority | ✅ Core verified | Authenticated roles, immediate/queued/commit-window classes, bounded anti-replay state, expiring one-shot approvals, and structured decisions are unit tested; credential adapter and queue execution remain open |
| Safety shim | 🟡 Core verified | Supervisor fault escalation reaches a simulated 20-actuator safe-state transport within the 100 ms bound; real KOS/MuJoCo transports remain open |
| Phase 2 plan + W1 inspection | 🟡 Foundation verified | Canonical JSON plan, bounded execution, authenticated gates, and a concrete inspection runner through authority/safety-shim are unit tested; the observation/actuator adapters are still in-memory, with mission control and MuJoCo coupling open |
| Fault-injected integrated MuJoCo | 🔴 | Runtime and the composed physics model remain separate processes |

## Phase 0 exit assessment

The documented exit criterion is:

> Robot (sim) streams health telemetry; killing a service triggers detection + logged recovery within a bounded time; PHA v0 reviewed.

The runtime tests prove fault escalation through the transport boundary, and the composed model proves K-Bot/W1 asset integration. They are not yet one closed-loop MuJoCo system. PHA review also requires a human sign-off. Therefore **Phase 0 remains open**.

## Prioritized next actions

### P0 — close Phase 0

1. **Connect the safety shim to composed MuJoCo.** Implement an `ActuatorTransport` that writes the 20 sanitized commands into the model and maps safe-state to zero control/braking.
2. **Review and approve the robot configuration manifest.** The checked-in manifest is validated and explicitly simulation-only; controls/hardware owners must review its limits before hardware use.
3. **Connect the shim to real KOS gRPC.** Make it the sole forwarding path after the simulated transport is green.
4. **Run closed-loop fault injection.** Stream simulated health, inject at least one service or sensor fault, and measure the MuJoCo actuator response bound.
5. **Record PHA v0 review.** Add reviewer, date, findings, dispositions, and sign-off; automation must not self-certify this step.

### P1 / Phase 2 — advance without claiming Phase 0 closure

6. ~~Implement the bounded command-authority core and two-step arming (TR-P4-042/TR-P4-050).~~ **Done.**
7. ~~Implement the bounded queued task-plan executor with preconditions, aborts, timeouts, and human gates.~~ **Core done.** Next, connect verified credentials, mission-control upload, commit-window contact authorization, and the safety shim.
8. Add the IMU staleness monitor and its fault-injection scenario (F-05).
9. ~~Add the first concrete `inspect_fixture` runner against W1, with its commands routed through authority and the safety shim.~~ **In-memory integration done.** Next, implement MuJoCo actuator/observation adapters and measure waypoint completion plus safe-state response in the composed scene.
10. Add session recording and the first mission-control plan/health display.

## Current risks

| Risk | Mitigation |
| --- | --- |
| Documentation claims exceed executable evidence | Keep status split into logic, sim, and hardware verification; update traceability with each test |
| Direct hardware control bypasses Trion safety | Make the safety shim the sole actuator command path before adding policy execution |
| Configuration remains duplicated upstream | Introduce the Trion manifest first; generate or adapt upstream-facing configuration from it |
| Simulation smoke test regresses | Keep W1 headless stepping in CI; add fault scenarios incrementally |
| Scope expands before safety closure | Defer broad perception, skills, and mission-control work until the vertical safety demonstration passes |

## Phase 0 closure checklist

- [x] Rust formatting, linting, tests, and walking demo are green.
- [x] W1 fixture scene loads and steps in CI.
- [x] Telemetry overflow and watchdog timing requirements have direct tests.
- [x] A simulation-only robot manifest and actuator command gate have direct tests.
- [x] K-Bot is composed into W1 from the pinned upstream assets.
- [x] Injected supervisor fault reaches a simulated actuator transport through the safety shim within 100 ms.
- [ ] Sanitized actuator commands drive the composed MuJoCo model.
- [ ] Injected fault produces a bounded zero-control/brake response in MuJoCo.
- [ ] PHA v0 has a recorded human review.
