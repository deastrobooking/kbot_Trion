//! Command tier: run-mode / safe-mode state machine (TR-P4-020..021). Future
//! residents: the three command classes — immediate / queued / commit-window
//! (TR-P4-042) — and command authentication, role-based authority, and
//! two-step arming (TR-P4-050).

pub mod gate;
pub mod safe_mode;
