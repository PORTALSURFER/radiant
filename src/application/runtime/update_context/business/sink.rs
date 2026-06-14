use std::{sync::Arc, time::Instant};

use super::work_context::current_business_task;

/// Worker-side sender for intermediate business-work events.
///
/// Values emitted through this sink are delivered back to the host application
/// through the normal message queue. The sink does not expose UI state mutation
/// or runtime internals to worker code.
pub struct BusinessEventSink<Event> {
    emit: Arc<dyn Fn(Event) -> bool + Send + Sync + 'static>,
}

impl<Event> Clone for BusinessEventSink<Event> {
    fn clone(&self) -> Self {
        Self {
            emit: Arc::clone(&self.emit),
        }
    }
}

impl<Event> BusinessEventSink<Event> {
    pub(super) fn new(emit: impl Fn(Event) -> bool + Send + Sync + 'static) -> Self {
        Self {
            emit: Arc::new(emit),
        }
    }

    /// Emit one intermediate event. Returns `false` if the runtime no longer
    /// accepts messages, for example after shutdown.
    pub fn emit(&self, event: Event) -> bool {
        self.record_emit();
        (self.emit)(event)
    }

    fn record_emit(&self) {
        if let Some(task) = current_business_task() {
            let now = Instant::now();
            let mut last_event = task
                .last_stream_event
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let gap = now.saturating_duration_since(*last_event);
            *last_event = now;
            task.diagnostics
                .record_business_stream_event(task.name, task.priority, gap);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::runtime::update_context::business::work_context::with_business_work_diagnostics;
    use crate::runtime::{RuntimeDiagnosticsRecorder, TaskPriority};

    #[test]
    fn emit_records_current_business_task_stream_event() {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        with_business_work_diagnostics(
            Arc::clone(&diagnostics),
            "stream-test",
            TaskPriority::Interactive,
            || {
                let sink = BusinessEventSink::new(|_: usize| true);
                assert!(sink.emit(1));
            },
        );

        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.business.stream_events, 1);
        assert!(snapshot.business.recent.iter().any(|event| event.state
            == crate::runtime::BusinessTaskDiagnosticState::StreamEvent
            && event.name == "stream-test"));
    }
}
