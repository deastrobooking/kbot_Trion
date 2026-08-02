//! Bounded-memory health telemetry bus (TR-P4-001, TR-P4-003).

use crate::time::RuntimeClock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthLevel {
    Nominal,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub service: String,
    pub level: HealthLevel,
    pub message: Option<String>,
    /// Milliseconds since runtime start on the robot's monotonic clock (TR-P4-002).
    pub monotonic_ms: u64,
}

/// Broadcast bus with a fixed channel capacity. Publishing never blocks and
/// never allocates unboundedly; reports that find no subscriber are dropped
/// and counted (TR-P4-003).
#[derive(Clone)]
pub struct HealthBus {
    tx: broadcast::Sender<HealthReport>,
    clock: RuntimeClock,
    dropped: Arc<AtomicU64>,
}

impl HealthBus {
    pub fn new(clock: RuntimeClock, capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            clock,
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn publish(&self, service: &str, level: HealthLevel, message: Option<String>) {
        let report = HealthReport {
            service: service.to_owned(),
            level,
            message,
            monotonic_ms: self.clock.now_ms(),
        };
        if self.tx.send(report).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<HealthReport> {
        self.tx.subscribe()
    }

    /// Reports dropped because no subscriber was listening (TR-P4-003).
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_reaches_subscriber_and_counts_drops() -> eyre::Result<()> {
        let bus = HealthBus::new(RuntimeClock::new(), 8);
        bus.publish("svc", HealthLevel::Nominal, None);
        assert_eq!(bus.dropped_count(), 1);

        let mut rx = bus.subscribe();
        bus.publish("svc", HealthLevel::Degraded, Some("warm".to_owned()));
        let report = rx.recv().await?;
        assert_eq!(report.service, "svc");
        assert_eq!(report.level, HealthLevel::Degraded);
        assert_eq!(bus.dropped_count(), 1);
        Ok(())
    }
}
