# TRION/src — Rust workspace

The Cargo workspace for all Trion on-robot and ground-segment code. One crate per architecture block of [PROJECT_PLAN.md §3](../docs/PROJECT_PLAN.md); crates are added to `Cargo.toml` `members` as they come online.

## Crates

| Crate | Status | Pillar | Purpose |
| --- | --- | --- | --- |
| [`trion-runtime`](trion-runtime/) | ✅ active | P4 | Safety-critical supervision: health telemetry, watchdogs, FDIR escalation, safe-mode. **Must never depend on perception/network/mission-control code (TR-P4-022).** |
| [`trion-skills`](trion-skills/) | 🟡 Phase 2 foundation | P1/P3 | Bounded task-plan executor, autonomy ceilings, human gates, and initial inspect/handle/connector execution envelopes; motion runners remain open |
| `trion-perception` | planned (Phase 2) | P2 | Localization, fixture recognition, worksite mapping — non-safety-critical tier |
| `mission-control` | planned (Phase 1) | P5 | Ground console: plan upload/review, telemetry dashboards, command classes/auth client |

Simulation assets and composition checks live outside this workspace in `TRION/sim/`; closed-loop fault injection is the next integration step.

## Crate-internal convention

Modules are grouped by architectural tier, one directory per tier, with `lib.rs` holding the tier map and flat re-exports of the public API. `trion-runtime` sets the pattern:

```
trion-runtime/src/
├── lib.rs          # tier map + public re-exports (API stays flat & stable)
├── time/           # single monotonic time base            (TR-P4-002)
├── telemetry/      # health bus, event log; later: downlink shaping, reports
│   ├── health.rs
│   └── events.rs
├── fdir/           # detect→isolate→recover→escalate; one file per monitor
│   ├── watchdog.rs         (F-01)
│   └── supervisor.rs
├── command/        # authority, safe-mode, manifest gate, transport shim
│   ├── authority.rs
│   ├── gate.rs
│   ├── kos_shim.rs
│   └── safe_mode.rs
├── config/         # versioned robot-manifest schema + validation
└── bin/            # runnable demos & tools
    └── walking_skeleton.rs
```

Growth rules:

- **New FDIR monitors** (F-02…F-10) are new files in `fdir/`, one per fault class, registered with the supervisor.
- **New tiers get new directories**, not new files at the root; `lib.rs` re-exports keep external imports (`trion_runtime::Watchdog`) unchanged.
- Every public item cites the requirement it implements (`TR-P4-0xx`) in its doc comment — that's what makes [REQ_TEST_TRACEABILITY.md](../docs/REQ_TEST_TRACEABILITY.md) auditable.
- Upstream K-Scale code style throughout: `cargo fmt`, `cargo clippy`, `tracing`, `eyre`, no `unwrap()`/`expect()` (TR-P4-070).

## Commands

```bash
cargo build                        # whole workspace
cargo test                         # requirement-traced tests
cargo clippy --all-targets         # lint gate
cargo run --bin walking_skeleton   # Phase 0 FDIR demo
```

The canonical Phase 2 plan format is documented in [`TASK_PLAN_FORMAT.md`](../docs/TASK_PLAN_FORMAT.md), with a checked-in W1 example under [`TRION/plans`](../plans/).
