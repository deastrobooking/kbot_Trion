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

/// Broadcast bus with a fixed channel capacity. Publishing never blocks or
/// allocates unboundedly. Reports with no subscriber and receiver-observed
/// overflow evictions are counted (TR-P4-003).
#[derive(Clone)]
pub struct HealthBus {
    tx: broadcast::Sender<HealthReport>,
    clock: RuntimeClock,
    dropped: Arc<AtomicU64>,
}

/// Receiver wrapper that accounts for reports evicted while this subscriber
/// was lagging. Tokio reports those evictions only when the receiver next
/// polls, so accounting belongs on the receive path (TR-P4-003).
pub struct HealthReceiver {
    rx: broadcast::Receiver<HealthReport>,
    dropped: Arc<AtomicU64>,
}

impl HealthReceiver {
    pub async fn recv(&mut self) -> Result<HealthReport, broadcast::error::RecvError> {
        match self.rx.recv().await {
            Err(broadcast::error::RecvError::Lagged(count)) => {
                self.dropped.fetch_add(count, Ordering::Relaxed);
                Err(broadcast::error::RecvError::Lagged(count))
            }
            result => result,
        }
    }
}

impl HealthBus {
    pub fn new(clock: RuntimeClock, capacity: usize) -> eyre::Result<Self> {
        if capacity == 0 {
            return Err(eyre::eyre!("health bus capacity must be greater than zero"));
        }
        let (tx, _) = broadcast::channel(capacity);
        Ok(Self {
            tx,
            clock,
            dropped: Arc::new(AtomicU64::new(0)),
        })
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

    pub fn subscribe(&self) -> HealthReceiver {
        HealthReceiver {
            rx: self.tx.subscribe(),
            dropped: Arc::clone(&self.dropped),
        }
    }

    /// Reports dropped because no subscriber listened or a receiver lagged.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_reaches_subscriber_and_counts_drops() -> eyre::Result<()> {
        let bus = HealthBus::new(RuntimeClock::new(), 2)?;
        bus.publish("svc", HealthLevel::Nominal, None);
        assert_eq!(bus.dropped_count(), 1);

        let mut rx = bus.subscribe();
        bus.publish("svc", HealthLevel::Nominal, None);
        bus.publish("svc", HealthLevel::Nominal, None);
        bus.publish("svc", HealthLevel::Degraded, Some("warm".to_owned()));
        assert!(matches!(
            rx.recv().await,
            Err(broadcast::error::RecvError::Lagged(1))
        ));
        assert_eq!(bus.dropped_count(), 2, "overflow drop must be counted");

        let report = rx.recv().await?;
        assert_eq!(report.service, "svc");
        assert_eq!(report.level, HealthLevel::Nominal);
        let report = rx.recv().await?;
        assert_eq!(report.level, HealthLevel::Degraded);
        Ok(())
    }

    #[test]
    fn zero_capacity_is_rejected() {
        assert!(HealthBus::new(RuntimeClock::new(), 0).is_err());
    }
}
