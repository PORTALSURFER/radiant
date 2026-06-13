use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::runtime::TaskPriority;

const RECENT_BUSINESS_EVENTS: usize = 32;
pub(crate) const SLOW_UPDATE_HANDLER_THRESHOLD: Duration = Duration::from_millis(50);

/// Snapshot of generic runtime diagnostics suitable for tests and debug panels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeDiagnostics {
    /// Runtime-owned business worker lifecycle diagnostics.
    pub business: BusinessRuntimeDiagnostics,
    /// UI/update responsiveness diagnostics recorded by the generic controller.
    pub ui: UiRuntimeDiagnostics,
}

/// Counters and recent events for runtime-managed business work.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BusinessRuntimeDiagnostics {
    /// Number of business tasks accepted into a runtime worker queue.
    pub queued: usize,
    /// Number of accepted business tasks that started running.
    pub started: usize,
    /// Number of business tasks that ran to normal completion.
    pub completed: usize,
    /// Number of business tasks that completed after cooperative cancellation was requested.
    pub cancelled: usize,
    /// Number of business tasks the runtime could not execute.
    pub failed: usize,
    /// Number of business tasks rejected before worker execution.
    pub rejected: usize,
    /// Number of business tasks currently running.
    pub running: usize,
    /// Longest observed delay between task submission and worker start.
    pub max_queue_delay: Duration,
    /// Longest observed worker execution duration.
    pub max_run_duration: Duration,
    /// Bounded recent lifecycle events, ordered oldest to newest.
    pub recent: Vec<BusinessTaskDiagnostic>,
}

/// One bounded business-task lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusinessTaskDiagnostic {
    /// Stable application-supplied task label.
    pub name: &'static str,
    /// Scheduling lane requested for the task.
    pub priority: TaskPriority,
    /// Lifecycle state recorded by this event.
    pub state: BusinessTaskDiagnosticState,
    /// Delay before worker start for start events.
    pub queue_delay: Option<Duration>,
    /// Worker duration for terminal events.
    pub run_duration: Option<Duration>,
}

/// Business-task lifecycle state recorded in runtime diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BusinessTaskDiagnosticState {
    /// Task was accepted into the worker queue.
    Queued,
    /// Task started executing on a business worker.
    Started,
    /// Task completed without a cancellation request.
    Completed,
    /// Task completed after cooperative cancellation had been requested.
    Cancelled,
    /// Task could not be accepted by the worker queue.
    Rejected,
}

/// UI-path responsiveness diagnostics recorded by the generic runtime controller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiRuntimeDiagnostics {
    /// Number of host update handlers observed by the controller.
    pub update_handlers: usize,
    /// Number of update handlers that exceeded the development slow-handler threshold.
    pub slow_update_handlers: usize,
    /// Longest observed update-handler duration.
    pub longest_update_handler: Duration,
    /// Most recent update handler that exceeded the slow-handler threshold.
    pub last_slow_update_handler: Option<UiUpdateHandlerDiagnostic>,
}

/// One slow update-handler diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiUpdateHandlerDiagnostic {
    /// Measured handler duration.
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BusinessTaskDiagnosticId(u64);

pub(crate) struct RuntimeDiagnosticsRecorder {
    state: Mutex<RuntimeDiagnosticsState>,
}

#[derive(Debug)]
struct RuntimeDiagnosticsState {
    snapshot: RuntimeDiagnostics,
    next_business_id: u64,
    recent_business: VecDeque<BusinessTaskDiagnostic>,
}

impl Default for RuntimeDiagnosticsRecorder {
    fn default() -> Self {
        Self {
            state: Mutex::new(RuntimeDiagnosticsState {
                snapshot: RuntimeDiagnostics::default(),
                next_business_id: 1,
                recent_business: VecDeque::with_capacity(RECENT_BUSINESS_EVENTS),
            }),
        }
    }
}

impl RuntimeDiagnosticsRecorder {
    pub(crate) fn snapshot(&self) -> RuntimeDiagnostics {
        let state = lock_diagnostics_state(&self.state);
        let mut snapshot = state.snapshot.clone();
        snapshot.business.recent = state.recent_business.iter().cloned().collect();
        snapshot
    }

    pub(crate) fn record_business_queued(
        &self,
        name: &'static str,
        priority: TaskPriority,
    ) -> BusinessTaskDiagnosticId {
        let mut state = lock_diagnostics_state(&self.state);
        let id = BusinessTaskDiagnosticId(state.next_business_id);
        state.next_business_id = state.next_business_id.saturating_add(1);
        state.snapshot.business.queued += 1;
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::Queued,
                queue_delay: None,
                run_duration: None,
            },
        );
        tracing::debug!(work.name = name, ?priority, "radiant business work queued");
        id
    }

    pub(crate) fn record_business_started(
        &self,
        name: &'static str,
        priority: TaskPriority,
        queue_delay: Duration,
    ) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.business.started += 1;
        state.snapshot.business.running += 1;
        state.snapshot.business.max_queue_delay =
            state.snapshot.business.max_queue_delay.max(queue_delay);
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::Started,
                queue_delay: Some(queue_delay),
                run_duration: None,
            },
        );
        tracing::debug!(
            work.name = name,
            ?priority,
            queue_delay_ms = queue_delay.as_secs_f64() * 1000.0,
            "radiant business work started"
        );
    }

    pub(crate) fn record_business_finished(
        &self,
        name: &'static str,
        priority: TaskPriority,
        state: BusinessTaskDiagnosticState,
        run_duration: Duration,
    ) {
        let mut diagnostics = lock_diagnostics_state(&self.state);
        diagnostics.snapshot.business.running =
            diagnostics.snapshot.business.running.saturating_sub(1);
        diagnostics.snapshot.business.max_run_duration = diagnostics
            .snapshot
            .business
            .max_run_duration
            .max(run_duration);
        match state {
            BusinessTaskDiagnosticState::Completed => diagnostics.snapshot.business.completed += 1,
            BusinessTaskDiagnosticState::Cancelled => diagnostics.snapshot.business.cancelled += 1,
            BusinessTaskDiagnosticState::Rejected => {
                diagnostics.snapshot.business.rejected += 1;
                diagnostics.snapshot.business.failed += 1;
            }
            BusinessTaskDiagnosticState::Queued | BusinessTaskDiagnosticState::Started => {}
        }
        push_business_event(
            &mut diagnostics,
            BusinessTaskDiagnostic {
                name,
                priority,
                state,
                queue_delay: None,
                run_duration: Some(run_duration),
            },
        );
        tracing::debug!(
            work.name = name,
            ?priority,
            ?state,
            run_duration_ms = run_duration.as_secs_f64() * 1000.0,
            "radiant business work finished"
        );
    }

    pub(crate) fn record_business_rejected(&self, name: &'static str, priority: TaskPriority) {
        self.record_business_finished(
            name,
            priority,
            BusinessTaskDiagnosticState::Rejected,
            Duration::ZERO,
        );
    }

    pub(crate) fn record_update_handler(&self, duration: Duration) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.ui.update_handlers += 1;
        state.snapshot.ui.longest_update_handler =
            state.snapshot.ui.longest_update_handler.max(duration);
        if duration >= SLOW_UPDATE_HANDLER_THRESHOLD {
            state.snapshot.ui.slow_update_handlers += 1;
            state.snapshot.ui.last_slow_update_handler =
                Some(UiUpdateHandlerDiagnostic { duration });
            tracing::warn!(
                update_duration_ms = duration.as_secs_f64() * 1000.0,
                threshold_ms = SLOW_UPDATE_HANDLER_THRESHOLD.as_secs_f64() * 1000.0,
                "radiant update handler exceeded development responsiveness threshold"
            );
        }
    }
}

pub(crate) fn elapsed_since(started: Instant) -> Duration {
    started.elapsed()
}

fn push_business_event(state: &mut RuntimeDiagnosticsState, event: BusinessTaskDiagnostic) {
    if state.recent_business.len() == RECENT_BUSINESS_EVENTS {
        state.recent_business.pop_front();
    }
    state.recent_business.push_back(event);
}

fn lock_diagnostics_state(
    state: &Mutex<RuntimeDiagnosticsState>,
) -> std::sync::MutexGuard<'_, RuntimeDiagnosticsState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
