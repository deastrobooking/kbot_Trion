//! trion-runtime — mission-critical runtime supervision for the Trion space
//! operations engineer (walking skeleton of the P4 pillar).
//!
//! Implements TR-P4-001..003 (health telemetry), TR-P4-010..012 (heartbeat
//! watchdog + restart escalation), TR-P4-020..022 (safe-mode), and TR-P4-030
//! (structured event log). See `TRION/docs/REQUIREMENTS.md`.
//!
//! Safety/control separation (TR-P4-022): this crate must never depend on
//! perception, network, or mission-control code.

pub mod clock;
pub mod events;
pub mod health;
pub mod safe_mode;
pub mod supervisor;
pub mod watchdog;

pub use clock::RuntimeClock;
pub use events::{EventKind, EventLog, EventRecord};
pub use health::{HealthBus, HealthLevel, HealthReport};
pub use safe_mode::{ModeController, RunMode};
pub use supervisor::{EscalationPolicy, ServiceController, Supervisor};
pub use watchdog::{HeartbeatHandle, HeartbeatPolicy, ServiceDown, Watchdog};
