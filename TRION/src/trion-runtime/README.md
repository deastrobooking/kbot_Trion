# trion-runtime

Mission-critical runtime supervision for the Trion space operations engineer — the walking skeleton of the P4 pillar in [PROJECT_PLAN.md](../../docs/PROJECT_PLAN.md).

Implements (traced to [REQUIREMENTS.md](../../docs/REQUIREMENTS.md)):

- **Health telemetry bus** — bounded broadcast channel, monotonic timestamps, counted drops (TR-P4-001…003)
- **Heartbeat watchdog** — per-service interval + miss limit, bounded-time detection (TR-P4-010, TR-P4-011)
- **FDIR escalation supervisor** — one bounded restart path per fault, windowed restart budget, then safe-mode (TR-P4-012, FDIR entry F-01)
- **Safe-mode state machine** — watch-channel broadcast to actuator services; exit only via explicit audited recovery command (TR-P4-020, TR-P4-021)
- **Structured event log** — bounded, JSONL-exportable evidence trail (TR-P4-030)

Safety/control separation (TR-P4-022): this crate must never grow dependencies on perception, network, or mission-control code.

```bash
cargo test                        # requirement-traced tests
cargo run --bin walking_skeleton  # Phase 0 exit-criteria demo (~8 s)
```

The demo simulates two KOS services: `imu-svc` crashes once and is restarted by FDIR; `actuator-svc` crashes repeatedly, exhausts its restart budget, and drives the runtime into safe-mode until an operator issues the recovery command.
