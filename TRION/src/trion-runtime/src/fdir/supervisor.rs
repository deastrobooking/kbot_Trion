//! FDIR escalation supervisor (TR-P4-012, FDIR entry F-01): one bounded
//! recovery path per fault, then safe-mode. No infinite retry loops.

use crate::time::{duration_ms, RuntimeClock};
use crate::telemetry::events::{EventKind, EventLog};
use crate::command::safe_mode::ModeController;
use crate::fdir::watchdog::ServiceDown;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, watch};

/// Restart hook for supervised services. Implementations must only *initiate*
/// the restart (spawn, signal the process manager) and return; liveness is
/// re-verified by the watchdog, not by the controller.
pub trait ServiceController: Send + Sync + 'static {
    fn restart(&self, service: &str) -> eyre::Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub struct EscalationPolicy {
    /// Restarts allowed per service within `window` before safe-mode.
    pub max_restarts: u32,
    pub window: Duration,
}

pub struct Supervisor<C: ServiceController> {
    controller: C,
    policy: EscalationPolicy,
    events: EventLog,
    modes: ModeController,
    clock: RuntimeClock,
    /// Restart timestamps per service; bounded at `max_restarts` entries.
    restart_history: HashMap<String, Vec<u64>>,
}

impl<C: ServiceController> Supervisor<C> {
    pub fn new(
        controller: C,
        policy: EscalationPolicy,
        events: EventLog,
        modes: ModeController,
        clock: RuntimeClock,
    ) -> Self {
        Self {
            controller,
            policy,
            events,
            modes,
            clock,
            restart_history: HashMap::new(),
        }
    }

    pub async fn run(mut self, mut faults: mpsc::Receiver<ServiceDown>, mut shutdown: watch::Receiver<bool>) {
        loop {
            tokio::select! {
                fault = faults.recv() => {
                    let Some(fault) = fault else { return };
                    self.handle(fault);
                }
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return;
                    }
                }
            }
        }
    }

    fn handle(&mut self, fault: ServiceDown) {
        self.events.record(
            EventKind::FaultDetected,
            Some(&fault.service),
            format!("missed heartbeats; silent for {} ms", fault.silent_for_ms),
        );
        if self.modes.is_safe() {
            // Already safe: log only. Recovery is human-commanded (TR-P4-021).
            return;
        }

        let now = self.clock.now_ms();
        let window_ms = duration_ms(self.policy.window);
        let history = self.restart_history.entry(fault.service.clone()).or_default();
        history.retain(|stamp| now.saturating_sub(*stamp) <= window_ms);

        let restarts_in_window = u32::try_from(history.len()).unwrap_or(u32::MAX);
        if restarts_in_window >= self.policy.max_restarts {
            self.modes.enter_safe(&format!(
                "service '{}' exceeded {} restarts within {} ms",
                fault.service, self.policy.max_restarts, window_ms
            ));
            return;
        }
        history.push(now);

        self.events.record(
            EventKind::RecoveryAttempted,
            Some(&fault.service),
            "restart requested",
        );
        match self.controller.restart(&fault.service) {
            Ok(()) => self.events.record(
                EventKind::RecoverySucceeded,
                Some(&fault.service),
                "restart initiated",
            ),
            Err(err) => {
                self.events.record(
                    EventKind::RecoveryFailed,
                    Some(&fault.service),
                    format!("restart failed: {err}"),
                );
                self.modes
                    .enter_safe(&format!("restart of '{}' failed", fault.service));
            }
        }
    }
}
