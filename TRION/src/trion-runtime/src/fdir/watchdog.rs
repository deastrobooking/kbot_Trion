//! Heartbeat watchdog (TR-P4-010, TR-P4-011). Services register a heartbeat
//! policy; the monitor loop emits a `ServiceDown` fault when a service's
//! silence exceeds `miss_limit × interval`, guaranteeing detection within
//! `(miss_limit + 1) × interval` plus one monitor tick.

use crate::time::{duration_ms, RuntimeClock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

#[derive(Debug, Clone, Copy)]
pub struct HeartbeatPolicy {
    pub interval: Duration,
    pub miss_limit: u32,
}

/// Handed to the supervised service; lock-free and cheap to beat from any
/// thread or task.
#[derive(Clone)]
pub struct HeartbeatHandle {
    last_beat_ms: Arc<AtomicU64>,
    clock: RuntimeClock,
}

impl HeartbeatHandle {
    pub fn beat(&self) {
        self.last_beat_ms.store(self.clock.now_ms(), Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDown {
    pub service: String,
    pub silent_for_ms: u64,
}

struct Entry {
    name: String,
    policy: HeartbeatPolicy,
    last_beat_ms: Arc<AtomicU64>,
    down_reported: bool,
}

pub struct Watchdog {
    clock: RuntimeClock,
    entries: Vec<Entry>,
    tick: Duration,
}

impl Watchdog {
    pub fn new(clock: RuntimeClock, tick: Duration) -> Self {
        Self {
            clock,
            entries: Vec::new(),
            tick,
        }
    }

    /// Register a service (TR-P4-010). Registration counts as the first beat.
    pub fn register(&mut self, name: &str, policy: HeartbeatPolicy) -> HeartbeatHandle {
        let cell = Arc::new(AtomicU64::new(self.clock.now_ms()));
        self.entries.push(Entry {
            name: name.to_owned(),
            policy,
            last_beat_ms: Arc::clone(&cell),
            down_reported: false,
        });
        HeartbeatHandle {
            last_beat_ms: cell,
            clock: self.clock,
        }
    }

    /// Monitor loop. Each fault is reported once per outage; a service that
    /// beats again re-arms its detection. Runs until `shutdown` flips true.
    pub async fn run(mut self, faults: mpsc::Sender<ServiceDown>, mut shutdown: watch::Receiver<bool>) {
        let mut ticker = tokio::time::interval(self.tick);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let now = self.clock.now_ms();
                    for entry in &mut self.entries {
                        let last = entry.last_beat_ms.load(Ordering::Relaxed);
                        let silent = now.saturating_sub(last);
                        let allowed = duration_ms(entry.policy.interval)
                            .saturating_mul(u64::from(entry.policy.miss_limit));
                        if silent > allowed {
                            if !entry.down_reported {
                                entry.down_reported = true;
                                let fault = ServiceDown {
                                    service: entry.name.clone(),
                                    silent_for_ms: silent,
                                };
                                if faults.send(fault).await.is_err() {
                                    return;
                                }
                            }
                        } else {
                            entry.down_reported = false;
                        }
                    }
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return;
                    }
                }
            }
        }
    }
}
