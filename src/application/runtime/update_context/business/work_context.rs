use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{
    application::CancellationToken,
    runtime::{RuntimeDiagnosticsRecorder, TaskPriority, elapsed_since},
};

/// Context supplied to a business worker closure.
#[derive(Clone)]
pub struct BusinessWorkContext {
    cancellation: Option<CancellationToken>,
    last_checkpoint: Arc<Mutex<Instant>>,
}

impl BusinessWorkContext {
    pub(super) fn new(cancellation: Option<CancellationToken>) -> Self {
        let now = Instant::now();
        Self {
            cancellation,
            last_checkpoint: Arc::new(Mutex::new(now)),
        }
    }

    /// Return whether cooperative cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
    }

    /// Return an error when cooperative cancellation has been requested.
    pub fn check_cancelled(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err(String::from("cancelled"))
        } else {
            Ok(())
        }
    }

    /// Record a cooperative checkpoint for diagnostics and cancellation-aware workers.
    pub fn checkpoint(&self) -> Result<(), String> {
        self.record_checkpoint();
        self.check_cancelled()
    }

    /// Yield the current thread and record a checkpoint when enough time has
    /// elapsed since the previous checkpoint.
    pub fn yield_if_elapsed(&self, max_elapsed: Duration) -> Result<(), String> {
        if self.elapsed_since_checkpoint() >= max_elapsed {
            thread::yield_now();
            self.record_checkpoint();
        }
        self.check_cancelled()
    }

    /// Return an error when too much time has elapsed since the previous
    /// checkpoint. Use this for interactive workers with strict responsiveness
    /// budgets.
    pub fn fail_if_over_budget(&self, budget: Duration) -> Result<(), String> {
        let elapsed = self.elapsed_since_checkpoint();
        if elapsed > budget {
            self.record_checkpoint();
            Err(format!(
                "business work exceeded checkpoint budget: elapsed_ms={:.3}, budget_ms={:.3}",
                elapsed.as_secs_f64() * 1000.0,
                budget.as_secs_f64() * 1000.0
            ))
        } else {
            self.check_cancelled()
        }
    }

    fn elapsed_since_checkpoint(&self) -> Duration {
        elapsed_since(*lock_instant(&self.last_checkpoint))
    }

    fn record_checkpoint(&self) {
        let now = Instant::now();
        let mut last_checkpoint = lock_instant(&self.last_checkpoint);
        let gap = now.saturating_duration_since(*last_checkpoint);
        *last_checkpoint = now;
        if let Some(task) = current_business_task() {
            record_current_business_checkpoint();
            task.diagnostics
                .record_business_checkpoint(task.name, task.priority, gap);
        }
    }
}

fn lock_instant(instant: &Mutex<Instant>) -> std::sync::MutexGuard<'_, Instant> {
    instant
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_records_current_business_task_diagnostics() {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        with_business_work_diagnostics(
            Arc::clone(&diagnostics),
            "checkpoint-test",
            TaskPriority::Interactive,
            || {
                BusinessWorkContext::new(None)
                    .checkpoint()
                    .expect("checkpoint succeeds");
            },
        );

        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.business.checkpoints, 1);
        assert!(snapshot.business.recent.iter().any(|event| event.state
            == crate::runtime::BusinessTaskDiagnosticState::Checkpoint
            && event.name == "checkpoint-test"));
    }

    #[test]
    fn fail_if_over_budget_reports_budget_error() {
        let context = BusinessWorkContext::new(None);
        std::thread::sleep(Duration::from_millis(2));

        let error = context
            .fail_if_over_budget(Duration::ZERO)
            .expect_err("zero budget should fail after elapsed time");

        assert!(error.contains("checkpoint budget"));
    }
}

#[derive(Clone)]
pub(crate) struct BusinessWorkDiagnosticScope {
    pub(crate) diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    pub(crate) name: &'static str,
    pub(crate) priority: TaskPriority,
    pub(crate) state: Arc<Mutex<BusinessWorkDiagnosticState>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct BusinessWorkDiagnosticSummary {
    pub(crate) checkpoints: usize,
    pub(crate) stream_events: usize,
}

#[derive(Debug)]
pub(crate) struct BusinessWorkDiagnosticState {
    last_stream_event: Instant,
    checkpoints: usize,
    stream_events: usize,
}

thread_local! {
    static CURRENT_BUSINESS_TASK: RefCell<Option<BusinessWorkDiagnosticScope>> = const { RefCell::new(None) };
}

pub(crate) fn with_business_work_diagnostics(
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    name: &'static str,
    priority: TaskPriority,
    work: impl FnOnce(),
) -> BusinessWorkDiagnosticSummary {
    let state = Arc::new(Mutex::new(BusinessWorkDiagnosticState {
        last_stream_event: Instant::now(),
        checkpoints: 0,
        stream_events: 0,
    }));
    let previous = CURRENT_BUSINESS_TASK.with(|current| {
        current.replace(Some(BusinessWorkDiagnosticScope {
            diagnostics,
            name,
            priority,
            state: Arc::clone(&state),
        }))
    });
    work();
    CURRENT_BUSINESS_TASK.with(|current| {
        current.replace(previous);
    });
    let state = lock_diagnostic_state(&state);
    BusinessWorkDiagnosticSummary {
        checkpoints: state.checkpoints,
        stream_events: state.stream_events,
    }
}

pub(crate) fn current_business_task() -> Option<BusinessWorkDiagnosticScope> {
    CURRENT_BUSINESS_TASK.with(|current| current.borrow().clone())
}

pub(crate) fn record_current_business_checkpoint() {
    if let Some(task) = current_business_task() {
        let mut state = lock_diagnostic_state(&task.state);
        state.checkpoints = state.checkpoints.saturating_add(1);
    }
}

pub(crate) fn record_current_business_stream_event() -> Option<Duration> {
    current_business_task().map(|task| {
        let now = Instant::now();
        let mut state = lock_diagnostic_state(&task.state);
        let gap = now.saturating_duration_since(state.last_stream_event);
        state.last_stream_event = now;
        state.stream_events = state.stream_events.saturating_add(1);
        gap
    })
}

fn lock_diagnostic_state(
    state: &Mutex<BusinessWorkDiagnosticState>,
) -> std::sync::MutexGuard<'_, BusinessWorkDiagnosticState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
