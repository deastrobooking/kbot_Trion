//! Structured, bounded event log with JSONL export (TR-P4-030). Events are
//! the evidence trail for FDIR behavior and post-mission reports.

use crate::clock::RuntimeClock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    ModeChange,
    FaultDetected,
    RecoveryAttempted,
    RecoverySucceeded,
    RecoveryFailed,
    SafeModeEntered,
    SafeModeExited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub monotonic_ms: u64,
    pub kind: EventKind,
    pub service: Option<String>,
    pub detail: String,
}

#[derive(Clone)]
pub struct EventLog {
    inner: Arc<Mutex<VecDeque<EventRecord>>>,
    capacity: usize,
    clock: RuntimeClock,
}

impl EventLog {
    pub fn new(clock: RuntimeClock, capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
            clock,
        }
    }

    pub fn record(&self, kind: EventKind, service: Option<&str>, detail: impl Into<String>) {
        let record = EventRecord {
            monotonic_ms: self.clock.now_ms(),
            kind,
            service: service.map(str::to_owned),
            detail: detail.into(),
        };
        tracing::info!(
            target: "trion::event",
            kind = ?record.kind,
            service = record.service.as_deref().unwrap_or("-"),
            t_ms = record.monotonic_ms,
            "{}",
            record.detail
        );
        // A poisoned lock means another thread panicked; keep recording
        // best-effort rather than losing the evidence trail.
        let mut queue = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if queue.len() == self.capacity {
            queue.pop_front();
        }
        queue.push_back(record);
    }

    pub fn snapshot(&self) -> Vec<EventRecord> {
        let queue = match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.iter().cloned().collect()
    }

    pub fn to_jsonl(&self) -> String {
        self.snapshot()
            .iter()
            .filter_map(|record| serde_json::to_string(record).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_is_bounded_and_exports_jsonl() {
        let log = EventLog::new(RuntimeClock::new(), 2);
        log.record(EventKind::FaultDetected, Some("svc"), "first");
        log.record(EventKind::RecoveryAttempted, Some("svc"), "second");
        log.record(EventKind::RecoverySucceeded, Some("svc"), "third");
        let snapshot = log.snapshot();
        assert_eq!(snapshot.len(), 2, "oldest entry must be evicted");
        assert_eq!(snapshot[0].detail, "second");
        assert_eq!(log.to_jsonl().lines().count(), 2);
    }
}
