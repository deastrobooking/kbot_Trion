# Trion Project Review & Next-Steps Audit

**Date:** 2026-08-02  
**Scope:** TRION layer only (`TRION/`) against [PROJECT_PLAN.md](PROJECT_PLAN.md) Phase 0 exit criteria.

## Executive summary

Trion has a green native Rust foundation, a requirement-traced P4 runtime skeleton, a loadable W1 worksite, and CI gates for Rust plus the MuJoCo scene. Phase 0 is not closed because the K-Bot model and runtime are not yet integrated into W1, safe-mode does not yet reach a simulated actuator path, and PHA v0 still needs human review.

The next milestone is one vertical safety demonstration: **K-Bot in W1 receives a command through the Trion safety gate; an injected fault causes a measured safe actuator response and a structured event record.**

## Verified status

| Deliverable | Status | Evidence / open remainder |
| --- | --- | --- |
| Native Rust gates | ✅ | `cargo fmt --check`, Clippy with warnings denied, and all tests pass |
| P4 walking skeleton | ✅ Logic skeleton | Health, watchdog, restart escalation, mode transition, explicit recovery, JSONL events |
| Telemetry bounds | ✅ Unit verified | Zero capacity rejected; unobserved and receiver-overflow drops counted |
| Watchdog bound | ✅ Unit verified | Sampling is constrained to the heartbeat interval; deterministic elapsed-time test covers TR-P4-011 |
| PHA, requirements, FDIR, traceability | ✅ v0 artifacts | PHA human review/sign-off remains open |
| W1 MuJoCo worksite | ✅ Fixture | Scene loads and steps; K-Bot model is not composed into it |
| TRION CI | ✅ Skeleton | Rust gates, runtime demo, and headless W1 smoke test |
| Command safety gate | 🟡 Core verified | Manifest validation, limit clamping, unknown/non-finite rejection, and safe-mode rejection are unit tested; KOS integration remains open |
| KOS safe-state path | 🔴 | No KOS shim or measured zero-torque/brake response |
| Fault-injected integrated sim | 🔴 | Runtime and MuJoCo remain separate demonstrations |

## Phase 0 exit assessment

The documented exit criterion is:

> Robot (sim) streams health telemetry; killing a service triggers detection + logged recovery within a bounded time; PHA v0 reviewed.

The synthetic runtime demo proves the supervision logic, and W1 proves the fixture scene is stable. They do not yet prove the criterion as one integrated system. PHA review also requires a human sign-off. Therefore **Phase 0 remains open**.

## Prioritized next actions

### P0 — close Phase 0

1. **Compose K-Bot into W1.** Pull the pinned `kscale-assets` model through the existing `ksim-kbot` submodule and create a Trion-owned composed scene.
2. **Review and approve the robot configuration manifest.** The checked-in manifest is validated and explicitly simulation-only; controls/hardware owners must review its limits before hardware use.
3. **Implement the KOS safety shim.** Wire the implemented command gate into the sole forwarding path and add the privileged zero-torque/brake safe-state operation.
4. **Connect runtime and simulation.** Stream simulated health, inject at least one service or sensor fault, and measure the safe response bound.
5. **Record PHA v0 review.** Add reviewer, date, findings, dispositions, and sign-off; automation must not self-certify this step.

### P1 — begin only after the vertical safety slice

6. Implement authenticated command classes and two-step arming (TR-P4-042/TR-P4-050).
7. Add the IMU staleness monitor and its fault-injection scenario (F-05).
8. Add the ONNX policy service, with every command routed through the safety shim.
9. Add session recording and the first mission-control health display.

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
- [ ] K-Bot is composed into W1.
- [ ] Simulated actuator commands pass through the Trion safety gate.
- [ ] Injected fault produces a bounded safe actuator response and event evidence.
- [ ] PHA v0 has a recorded human review.
