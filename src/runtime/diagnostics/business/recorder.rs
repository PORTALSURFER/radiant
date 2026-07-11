use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

use super::models::RECENT_BUSINESS_EVENTS;
use super::{
    BusinessRuntimeDiagnostics, BusinessTaskDiagnostic, BusinessTaskDiagnosticState,
    RuntimeDiagnostics,
};
use crate::runtime::TaskPriority;

mod ui;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BusinessTaskDiagnosticId(u64);

pub(crate) struct RuntimeDiagnosticsRecorder {
    pub(super) state: Mutex<RuntimeDiagnosticsState>,
}

#[derive(Debug)]
pub(super) struct RuntimeDiagnosticsState {
    pub(super) snapshot: RuntimeDiagnostics,
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

    pub(crate) fn record_message_queue_depth(&self, pending_messages: usize, stream_slots: usize) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.queue.current_pending_messages = pending_messages;
        state.snapshot.queue.max_pending_messages = state
            .snapshot
            .queue
            .max_pending_messages
            .max(pending_messages);
        state.snapshot.queue.current_pending_stream_slots = stream_slots;
        state.snapshot.queue.max_pending_stream_slots = state
            .snapshot
            .queue
            .max_pending_stream_slots
            .max(stream_slots);
    }

    pub(crate) fn record_stream_message_coalesced(&self) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.queue.stream_events_coalesced += 1;
    }

    pub(crate) fn record_stream_message_stale(&self) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.queue.stream_events_stale += 1;
    }

    pub(crate) fn record_stream_message_dropped(&self) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.queue.stream_events_dropped += 1;
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
                checkpoint_gap: None,
                stream_event_gap: None,
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
        update_priority_queue_delay(&mut state.snapshot.business, priority, queue_delay);
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::Started,
                queue_delay: Some(queue_delay),
                run_duration: None,
                checkpoint_gap: None,
                stream_event_gap: None,
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
        update_priority_run_duration(&mut diagnostics.snapshot.business, priority, run_duration);
        match state {
            BusinessTaskDiagnosticState::Completed => diagnostics.snapshot.business.completed += 1,
            BusinessTaskDiagnosticState::Cancelled => diagnostics.snapshot.business.cancelled += 1,
            BusinessTaskDiagnosticState::Panicked => diagnostics.snapshot.business.failed += 1,
            BusinessTaskDiagnosticState::Rejected => {
                diagnostics.snapshot.business.rejected += 1;
                diagnostics.snapshot.business.failed += 1;
            }
            BusinessTaskDiagnosticState::Queued
            | BusinessTaskDiagnosticState::Started
            | BusinessTaskDiagnosticState::Checkpoint
            | BusinessTaskDiagnosticState::StreamEvent
            | BusinessTaskDiagnosticState::MissingCheckpoint
            | BusinessTaskDiagnosticState::MissingStreamEvent => {}
        }
        push_business_event(
            &mut diagnostics,
            BusinessTaskDiagnostic {
                name,
                priority,
                state,
                queue_delay: None,
                run_duration: Some(run_duration),
                checkpoint_gap: None,
                stream_event_gap: None,
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

    pub(crate) fn record_business_checkpoint(
        &self,
        name: &'static str,
        priority: TaskPriority,
        checkpoint_gap: Duration,
    ) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.business.checkpoints += 1;
        state.snapshot.business.max_checkpoint_gap = state
            .snapshot
            .business
            .max_checkpoint_gap
            .max(checkpoint_gap);
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::Checkpoint,
                queue_delay: None,
                run_duration: None,
                checkpoint_gap: Some(checkpoint_gap),
                stream_event_gap: None,
            },
        );
    }

    pub(crate) fn record_business_stream_event(
        &self,
        name: &'static str,
        priority: TaskPriority,
        stream_event_gap: Duration,
    ) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.business.stream_events += 1;
        state.snapshot.business.max_stream_event_gap = state
            .snapshot
            .business
            .max_stream_event_gap
            .max(stream_event_gap);
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::StreamEvent,
                queue_delay: None,
                run_duration: None,
                checkpoint_gap: None,
                stream_event_gap: Some(stream_event_gap),
            },
        );
    }

    pub(crate) fn record_business_missing_checkpoint(
        &self,
        name: &'static str,
        priority: TaskPriority,
        run_duration: Duration,
    ) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.business.missing_checkpoint_warnings += 1;
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::MissingCheckpoint,
                queue_delay: None,
                run_duration: Some(run_duration),
                checkpoint_gap: None,
                stream_event_gap: None,
            },
        );
        tracing::warn!(
            work.name = name,
            ?priority,
            run_duration_ms = run_duration.as_secs_f64() * 1000.0,
            "radiant business work exceeded checkpoint warning threshold without reporting a checkpoint"
        );
    }

    pub(crate) fn record_business_missing_stream_event(
        &self,
        name: &'static str,
        priority: TaskPriority,
        run_duration: Duration,
    ) {
        let mut state = lock_diagnostics_state(&self.state);
        state.snapshot.business.missing_stream_event_warnings += 1;
        push_business_event(
            &mut state,
            BusinessTaskDiagnostic {
                name,
                priority,
                state: BusinessTaskDiagnosticState::MissingStreamEvent,
                queue_delay: None,
                run_duration: Some(run_duration),
                checkpoint_gap: None,
                stream_event_gap: None,
            },
        );
        tracing::warn!(
            work.name = name,
            ?priority,
            run_duration_ms = run_duration.as_secs_f64() * 1000.0,
            "radiant business work exceeded stream-event warning threshold without emitting progress"
        );
    }
}

fn update_priority_queue_delay(
    diagnostics: &mut BusinessRuntimeDiagnostics,
    priority: TaskPriority,
    queue_delay: Duration,
) {
    match priority {
        TaskPriority::Interactive => {
            diagnostics.max_interactive_queue_delay =
                diagnostics.max_interactive_queue_delay.max(queue_delay);
        }
        TaskPriority::Background => {
            diagnostics.max_background_queue_delay =
                diagnostics.max_background_queue_delay.max(queue_delay);
        }
        TaskPriority::BlockingIo => {
            diagnostics.max_blocking_io_queue_delay =
                diagnostics.max_blocking_io_queue_delay.max(queue_delay);
        }
        TaskPriority::Idle => {
            diagnostics.max_idle_queue_delay = diagnostics.max_idle_queue_delay.max(queue_delay);
        }
    }
}

fn update_priority_run_duration(
    diagnostics: &mut BusinessRuntimeDiagnostics,
    priority: TaskPriority,
    run_duration: Duration,
) {
    match priority {
        TaskPriority::Interactive => {
            diagnostics.max_interactive_run_duration =
                diagnostics.max_interactive_run_duration.max(run_duration);
        }
        TaskPriority::Background => {
            diagnostics.max_background_run_duration =
                diagnostics.max_background_run_duration.max(run_duration);
        }
        TaskPriority::BlockingIo => {
            diagnostics.max_blocking_io_run_duration =
                diagnostics.max_blocking_io_run_duration.max(run_duration);
        }
        TaskPriority::Idle => {
            diagnostics.max_idle_run_duration = diagnostics.max_idle_run_duration.max(run_duration);
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

pub(super) fn lock_diagnostics_state(
    state: &Mutex<RuntimeDiagnosticsState>,
) -> std::sync::MutexGuard<'_, RuntimeDiagnosticsState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
