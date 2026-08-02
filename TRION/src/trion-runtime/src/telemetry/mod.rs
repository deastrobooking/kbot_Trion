//! Telemetry tier: the health bus (live system state, TR-P4-001..003) and the
//! structured event log (the evidence trail for FDIR behavior and
//! post-mission reports, TR-P4-030). Future residents: telemetry downlink
//! shaping for degraded comms modes (TR-P4-041) and the post-mission report
//! generator.

pub mod events;
pub mod health;
