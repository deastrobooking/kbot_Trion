//! trion-runtime — mission-critical runtime supervision for the Trion space
//! operations engineer (the P4 pillar of TRION/docs/PROJECT_PLAN.md).
//!
//! Module map (one directory per architectural tier):
//!
//! | Module      | Tier                                            | Requirements |
//! |-------------|--------------------------------------------------|--------------|
//! | [`time`]     | Single monotonic time base                       | TR-P4-002    |
//! | [`telemetry`]| Health bus + structured event log                | TR-P4-001..003, TR-P4-030 |
//! | [`fdir`]     | Watchdog + escalation supervisor                 | TR-P4-010..013 |
//! | [`command`]  | Safe-mode; later command classes + authentication| TR-P4-020..021 (042, 050 future) |
//!
//! Safety/control separation (TR-P4-022): this crate must never depend on
//! perception, network, or mission-control code.

pub mod command;
pub mod config;
pub mod fdir;
pub mod telemetry;
pub mod time;

pub use command::gate::{ActuatorCommand, CommandGate, GateOutcome};
pub use command::safe_mode::{ModeController, RunMode};
pub use config::{ActuatorConfig, RobotManifest};
pub use fdir::supervisor::{EscalationPolicy, ServiceController, Supervisor};
pub use fdir::watchdog::{HeartbeatHandle, HeartbeatPolicy, ServiceDown, Watchdog};
pub use telemetry::events::{EventKind, EventLog, EventRecord};
pub use telemetry::health::{HealthBus, HealthLevel, HealthReceiver, HealthReport};
pub use time::RuntimeClock;
