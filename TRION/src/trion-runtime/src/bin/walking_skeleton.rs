//! Phase 0 exit-criteria demo (see TRION/docs/PROJECT_PLAN.md):
//! two simulated services stream health telemetry; the IMU service crashes
//! once and is restarted by FDIR; the actuator service crashes repeatedly and
//! escalates to safe-mode; an operator then issues the explicit recovery
//! command. The structured event log prints at the end.

use eyre::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use trion_runtime::{
    EscalationPolicy, EventLog, HealthBus, HealthLevel, HeartbeatHandle, HeartbeatPolicy,
    ModeController, RuntimeClock, ServiceController, Supervisor, Watchdog,
};

/// A simulated KOS service: beats its heartbeat and publishes health until an
/// optional simulated crash time.
fn spawn_service(
    name: String,
    handle: HeartbeatHandle,
    bus: HealthBus,
    lifetime: Option<Duration>,
) {
    tokio::spawn(async move {
        let started = tokio::time::Instant::now();
        let mut ticker = tokio::time::interval(Duration::from_millis(50));
        loop {
            ticker.tick().await;
            if let Some(lifetime) = lifetime {
                if started.elapsed() >= lifetime {
                    bus.publish(
                        &name,
                        HealthLevel::Critical,
                        Some("simulated crash".to_owned()),
                    );
                    return;
                }
            }
            handle.beat();
            bus.publish(&name, HealthLevel::Nominal, None);
        }
    });
}

/// Demo restart hook: respawns a service task. The IMU service comes back
/// healthy; the actuator service is configured to keep crashing so the
/// escalation path is exercised.
struct DemoController {
    services: HashMap<String, (HeartbeatHandle, Option<Duration>)>,
    bus: HealthBus,
}

impl ServiceController for DemoController {
    fn restart(&self, service: &str) -> eyre::Result<()> {
        let (handle, lifetime) = self
            .services
            .get(service)
            .ok_or_else(|| eyre::eyre!("unknown service '{service}'"))?;
        handle.beat();
        spawn_service(
            service.to_owned(),
            handle.clone(),
            self.bus.clone(),
            *lifetime,
        );
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let clock = RuntimeClock::new();
    let bus = HealthBus::new(clock, 256)?;
    let events = EventLog::new(clock, 1024)?;
    let modes = ModeController::new(events.clone());

    let mut watchdog = Watchdog::new(clock, Duration::from_millis(25))?;
    let policy = HeartbeatPolicy {
        interval: Duration::from_millis(50),
        miss_limit: 4,
    };
    let imu = watchdog.register("imu-svc", policy)?;
    let actuator = watchdog.register("actuator-svc", policy)?;

    // Initial spawns: imu-svc crashes at t=2 s (its restart is healthy);
    // actuator-svc crashes at t=4 s and every restart dies again after 300 ms.
    spawn_service(
        "imu-svc".to_owned(),
        imu.clone(),
        bus.clone(),
        Some(Duration::from_secs(2)),
    );
    spawn_service(
        "actuator-svc".to_owned(),
        actuator.clone(),
        bus.clone(),
        Some(Duration::from_secs(4)),
    );

    let mut restart_specs = HashMap::new();
    restart_specs.insert("imu-svc".to_owned(), (imu, None));
    restart_specs.insert(
        "actuator-svc".to_owned(),
        (actuator, Some(Duration::from_millis(300))),
    );
    let controller = DemoController {
        services: restart_specs,
        bus: bus.clone(),
    };

    let (fault_tx, fault_rx) = mpsc::channel(16);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let supervisor = Supervisor::new(
        controller,
        EscalationPolicy {
            max_restarts: 2,
            window: Duration::from_secs(10),
        },
        events.clone(),
        modes.clone(),
        clock,
    );
    let watchdog_task = tokio::spawn(watchdog.run(fault_tx, shutdown_rx.clone()));
    let supervisor_task = tokio::spawn(supervisor.run(fault_rx, shutdown_rx.clone()));

    // Ground-console stand-in: log health level transitions only.
    let mut health_rx = bus.subscribe();
    let health_task = tokio::spawn(async move {
        let mut last: HashMap<String, HealthLevel> = HashMap::new();
        loop {
            match health_rx.recv().await {
                Ok(report) => {
                    if last.get(&report.service) != Some(&report.level) {
                        tracing::info!(
                            target: "trion::health",
                            service = %report.service,
                            level = ?report.level,
                            t_ms = report.monotonic_ms,
                            "health level change"
                        );
                        last.insert(report.service.clone(), report.level);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    });

    let mut mode_rx = modes.subscribe();
    let mode_task = tokio::spawn(async move {
        while mode_rx.changed().await.is_ok() {
            let mode = *mode_rx.borrow();
            tracing::warn!(target: "trion::mode", ?mode, "run-mode transition");
        }
    });

    tokio::time::sleep(Duration::from_secs(8)).await;

    if modes.is_safe() {
        tracing::info!("operator issues explicit recovery command (TR-P4-021)");
        modes.recover("demo-operator");
    }

    let _ = shutdown_tx.send(true);
    let _ = watchdog_task.await;
    let _ = supervisor_task.await;
    health_task.abort();
    mode_task.abort();

    println!("\n=== structured event log (JSONL, TR-P4-030) ===");
    println!("{}", events.to_jsonl());
    println!(
        "\nhealth reports dropped (unobserved or receiver overflow, TR-P4-003): {}",
        bus.dropped_count()
    );
    Ok(())
}
