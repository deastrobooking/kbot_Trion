//! FDIR tier: detect → isolate → recover → escalate (TR-P4-010..013).
//! Today: heartbeat watchdog (F-01) and the escalation supervisor. Future
//! residents: the remaining FDIR monitors from FDIR_MATRIX.md — actuator
//! thermal/current (F-02/F-03), CAN loss (F-04), IMU staleness (F-05),
//! frame-overrun (F-06), comms loss (F-07), power (F-08).

pub mod supervisor;
pub mod watchdog;
