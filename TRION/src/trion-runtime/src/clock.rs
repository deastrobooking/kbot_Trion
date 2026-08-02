//! Single monotonic time base for the robot (TR-P4-002). All telemetry and
//! event timestamps derive from this clock; mapping to wall time happens in
//! the ground segment at session start.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeClock {
    start: Instant,
}

impl RuntimeClock {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Milliseconds since runtime start, from the OS monotonic clock.
    pub fn now_ms(&self) -> u64 {
        duration_ms(self.start.elapsed())
    }
}

impl Default for RuntimeClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Saturating milliseconds conversion (bounded arithmetic, TR-P4-070).
pub(crate) fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
