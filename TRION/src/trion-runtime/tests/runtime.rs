//! Requirement-verifying tests. Each test names the TR-P4 requirement it
//! verifies (traceability, see TRION/docs/REQUIREMENTS.md).

use std::time::Duration;
use tokio::sync::{mpsc, watch};
use trion_runtime::{
    EscalationPolicy, EventLog, HeartbeatPolicy, ModeController, RunMode, RuntimeClock,
    ServiceController, ServiceDown, Supervisor, Watchdog,
};

struct OkController;

impl ServiceController for OkController {
    fn restart(&self, _service: &str) -> eyre::Result<()> {
        Ok(())
    }
}

/// TR-P4-011: missed heartbeats are detected within (miss_limit + 1) × interval.
#[tokio::test]
async fn missed_heartbeat_detected_within_bound() -> eyre::Result<()> {
    let clock = RuntimeClock::new();
    let mut watchdog = Watchdog::new(clock, Duration::from_millis(10));
    // interval 20 ms, miss_limit 3 → detection bound (3 + 1) × 20 = 80 ms; the
    // 200 ms timeout leaves slack for scheduler jitter while staying bounded.
    let _handle = watchdog.register(
        "svc",
        HeartbeatPolicy {
            interval: Duration::from_millis(20),
            miss_limit: 3,
        },
    );
    let (fault_tx, mut fault_rx) = mpsc::channel(4);
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(watchdog.run(fault_tx, shutdown_rx));

    let fault = tokio::time::timeout(Duration::from_millis(200), fault_rx.recv())
        .await?
        .ok_or_else(|| eyre::eyre!("fault channel closed before detection"))?;
    assert_eq!(fault.service, "svc");
    assert!(
        fault.silent_for_ms > 60,
        "silence must exceed miss_limit × interval"
    );
    Ok(())
}

/// TR-P4-010/011: a beating service never trips the watchdog.
#[tokio::test]
async fn healthy_service_produces_no_fault() -> eyre::Result<()> {
    let clock = RuntimeClock::new();
    let mut watchdog = Watchdog::new(clock, Duration::from_millis(10));
    let handle = watchdog.register(
        "svc",
        HeartbeatPolicy {
            interval: Duration::from_millis(20),
            miss_limit: 3,
        },
    );
    let (fault_tx, mut fault_rx) = mpsc::channel(4);
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(watchdog.run(fault_tx, shutdown_rx));

    let beater = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            handle.beat();
        }
    });

    let outcome = tokio::time::timeout(Duration::from_millis(150), fault_rx.recv()).await;
    beater.abort();
    assert!(outcome.is_err(), "no fault expected from a beating service");
    Ok(())
}

/// TR-P4-012: restarts beyond the escalation policy enter safe-mode.
/// TR-P4-021: safe-mode exits only on an explicit recovery command.
#[tokio::test]
async fn repeated_faults_escalate_to_safe_mode_and_recovery_is_explicit() -> eyre::Result<()> {
    let clock = RuntimeClock::new();
    let events = EventLog::new(clock, 64);
    let modes = ModeController::new(events.clone());
    let supervisor = Supervisor::new(
        OkController,
        EscalationPolicy {
            max_restarts: 2,
            window: Duration::from_secs(10),
        },
        events.clone(),
        modes.clone(),
        clock,
    );
    let (fault_tx, fault_rx) = mpsc::channel(8);
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(supervisor.run(fault_rx, shutdown_rx));

    for _ in 0..3 {
        fault_tx
            .send(ServiceDown {
                service: "svc".to_owned(),
                silent_for_ms: 100,
            })
            .await?;
    }

    let mut mode_rx = modes.subscribe();
    tokio::time::timeout(Duration::from_millis(500), async {
        while *mode_rx.borrow() != RunMode::Safe {
            if mode_rx.changed().await.is_err() {
                break;
            }
        }
    })
    .await?;
    assert!(modes.is_safe(), "third fault must escalate to safe-mode");

    // Safe-mode does not exit on its own; only the explicit command clears it.
    assert!(modes.is_safe());
    modes.recover("test-operator");
    assert!(!modes.is_safe(), "explicit recovery must exit safe-mode");

    let log = events.to_jsonl();
    assert!(log.contains("SafeModeEntered"), "event log must show entry");
    assert!(
        log.contains("SafeModeExited"),
        "event log must show audited exit"
    );
    Ok(())
}
