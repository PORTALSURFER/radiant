use std::time::Duration;

use crate::runtime::TaskPriority;

pub(super) const RECENT_BUSINESS_EVENTS: usize = 32;
/// Default warn-only threshold for UI update-handler responsiveness diagnostics.
pub const DEFAULT_SLOW_UPDATE_HANDLER_THRESHOLD: Duration = Duration::from_millis(50);
/// Standard guidance attached to slow update-handler diagnostics.
pub const SLOW_UPDATE_HANDLER_GUIDANCE: &str =
    "move business work into context.business() or a typed platform service";

/// Snapshot of generic runtime diagnostics suitable for tests and debug panels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeDiagnostics {
    /// Runtime message-queue pressure and coalescing diagnostics.
    pub queue: RuntimeMessageQueueDiagnostics,
    /// Runtime-owned business worker lifecycle diagnostics.
    pub business: BusinessRuntimeDiagnostics,
    /// UI/update responsiveness diagnostics recorded by the generic controller.
    pub ui: UiRuntimeDiagnostics,
}

/// Counters for runtime-delivered message queues.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeMessageQueueDiagnostics {
    /// Number of pending runtime-delivered messages currently waiting for the UI loop.
    pub current_pending_messages: usize,
    /// Largest pending runtime-delivered message depth observed since startup.
    pub max_pending_messages: usize,
    /// Number of coalescible stream slots currently represented in the pending queue.
    pub current_pending_stream_slots: usize,
    /// Largest coalescible stream-slot count observed since startup.
    pub max_pending_stream_slots: usize,
    /// Number of stream events that replaced an older pending event for the same slot.
    pub stream_events_coalesced: usize,
    /// Number of stream events dropped because their stream slot was no longer live.
    pub stream_events_stale: usize,
    /// Number of stream events dropped because the runtime was no longer accepting messages.
    pub stream_events_dropped: usize,
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
    /// Longest observed interactive-task queue delay.
    pub max_interactive_queue_delay: Duration,
    /// Longest observed background-task queue delay.
    pub max_background_queue_delay: Duration,
    /// Longest observed blocking-IO-task queue delay.
    pub max_blocking_io_queue_delay: Duration,
    /// Longest observed idle-task queue delay.
    pub max_idle_queue_delay: Duration,
    /// Longest observed worker execution duration.
    pub max_run_duration: Duration,
    /// Longest observed interactive worker execution duration.
    pub max_interactive_run_duration: Duration,
    /// Longest observed background worker execution duration.
    pub max_background_run_duration: Duration,
    /// Longest observed blocking-IO worker execution duration.
    pub max_blocking_io_run_duration: Duration,
    /// Longest observed idle worker execution duration.
    pub max_idle_run_duration: Duration,
    /// Number of cooperative worker checkpoints reported by business tasks.
    pub checkpoints: usize,
    /// Longest duration observed between cooperative checkpoints.
    pub max_checkpoint_gap: Duration,
    /// Number of intermediate events emitted by streaming business tasks.
    pub stream_events: usize,
    /// Longest duration observed between emitted stream events.
    pub max_stream_event_gap: Duration,
    /// Number of completed interactive tasks that exceeded the checkpoint warning threshold without a checkpoint.
    pub missing_checkpoint_warnings: usize,
    /// Number of completed tasks that exceeded the stream-event warning threshold without emitting an intermediate event.
    pub missing_stream_event_warnings: usize,
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
    /// Duration since the previous cooperative checkpoint for checkpoint events.
    pub checkpoint_gap: Option<Duration>,
    /// Duration since the previous stream event for stream-event diagnostics.
    pub stream_event_gap: Option<Duration>,
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
    /// Task panicked while executing on a worker thread.
    Panicked,
    /// Task could not be accepted by the worker queue.
    Rejected,
    /// Task reported a cooperative checkpoint.
    Checkpoint,
    /// Streaming task emitted one intermediate event.
    StreamEvent,
    /// Task exceeded the checkpoint warning threshold without reporting a checkpoint.
    MissingCheckpoint,
    /// Task exceeded the stream-event warning threshold without emitting progress.
    MissingStreamEvent,
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
    /// Configured slow-handler threshold.
    pub threshold: Duration,
    /// Runtime bridge type that executed the handler.
    pub handler: &'static str,
    /// Host message type reduced by the handler.
    pub message: &'static str,
    /// Developer guidance for fixing the slow handler.
    pub guidance: &'static str,
}

impl UiUpdateHandlerDiagnostic {
    /// Return the deterministic failure text used by strict diagnostics mode.
    pub fn failure_message(&self) -> String {
        format!(
            "radiant update handler exceeded configured responsiveness threshold: handler={}, message={}, elapsed_ms={:.3}, threshold_ms={:.3}; {}",
            self.handler,
            self.message,
            self.duration.as_secs_f64() * 1000.0,
            self.threshold.as_secs_f64() * 1000.0,
            self.guidance
        )
    }
}

/// Behavior for update-handler diagnostics when a handler exceeds the threshold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UiUpdateHandlerDiagnosticsMode {
    /// Record the slow handler and emit a warning log.
    #[default]
    Warn,
    /// Record the slow handler and panic to fail a test or development run.
    Panic,
}

/// Runtime policy for measuring UI update-handler duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiUpdateHandlerDiagnosticsPolicy {
    threshold: Option<Duration>,
    mode: UiUpdateHandlerDiagnosticsMode,
}

impl UiUpdateHandlerDiagnosticsPolicy {
    /// Return the default warn-only development diagnostics policy.
    pub const fn warn_default() -> Self {
        Self::warn_at(DEFAULT_SLOW_UPDATE_HANDLER_THRESHOLD)
    }

    /// Record and warn when update handlers meet or exceed `threshold`.
    pub const fn warn_at(threshold: Duration) -> Self {
        Self {
            threshold: Some(threshold),
            mode: UiUpdateHandlerDiagnosticsMode::Warn,
        }
    }

    /// Record and panic when update handlers meet or exceed `threshold`.
    pub const fn panic_at(threshold: Duration) -> Self {
        Self {
            threshold: Some(threshold),
            mode: UiUpdateHandlerDiagnosticsMode::Panic,
        }
    }

    /// Disable update-handler duration measurement for this runtime.
    pub const fn disabled() -> Self {
        Self {
            threshold: None,
            mode: UiUpdateHandlerDiagnosticsMode::Warn,
        }
    }

    /// Return the active threshold, or `None` when diagnostics are disabled.
    pub const fn threshold(self) -> Option<Duration> {
        self.threshold
    }

    /// Return the configured slow-handler action.
    pub const fn mode(self) -> UiUpdateHandlerDiagnosticsMode {
        self.mode
    }
}

impl Default for UiUpdateHandlerDiagnosticsPolicy {
    fn default() -> Self {
        if cfg!(debug_assertions) {
            Self::warn_default()
        } else {
            Self::disabled()
        }
    }
}
