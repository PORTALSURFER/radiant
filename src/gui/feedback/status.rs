const DEFAULT_STATUS_LINE_LIMIT: usize = 5;

/// Bounded one-line status message history for status bars and compact logs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineLog {
    entries: Vec<StatusLineEntry>,
    limit: usize,
}

/// One status-line event from a named UI, worker, or system source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusLineEntry {
    source: String,
    message: String,
}

impl StatusLineLog {
    /// Build a status-line log retaining at most `limit` entries.
    pub fn new(limit: usize) -> Self {
        Self {
            entries: vec![StatusLineEntry::new("system", "Ready")],
            limit: limit.max(1),
        }
    }

    /// Publish one compact status message from a named source.
    pub fn publish(&mut self, source: impl Into<String>, message: impl Into<String>) {
        self.entries
            .push(StatusLineEntry::new(source.into(), message.into()));
        let overflow = self.entries.len().saturating_sub(self.limit);
        if overflow > 0 {
            self.entries.drain(0..overflow);
        }
    }

    /// Return the latest status line formatted as `source: message`.
    pub fn latest(&self) -> String {
        self.entries
            .last()
            .map(StatusLineEntry::line)
            .unwrap_or_else(|| "system: Ready".to_string())
    }

    /// Return recent lines newest-first.
    pub fn recent_lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .rev()
            .map(StatusLineEntry::line)
            .collect()
    }

    /// Return the number of retained entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether this log has no retained entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for StatusLineLog {
    fn default() -> Self {
        Self::new(DEFAULT_STATUS_LINE_LIMIT)
    }
}

impl StatusLineEntry {
    /// Build a status-line entry from source and message text.
    pub fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    /// Return the status producer label.
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// Return the one-line status message.
    pub fn message(&self) -> &str {
        self.message.as_str()
    }

    /// Format this entry as `source: message` for compact status bars.
    pub fn line(&self) -> String {
        format!("{}: {}", self.source, self.message)
    }
}

/// Summary for recoverable background work surfaced in a sidebar, panel, or status region.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RecoverySummary {
    /// Whether recovery work is still running in the background.
    pub in_progress: bool,
    /// Number of completed recovery entries currently visible or retained for review.
    pub entry_count: usize,
    /// Number of entries awaiting explicit user action.
    pub retained_count: usize,
}

/// Generic health state for compact status chips and panel summaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HealthState {
    /// The represented subsystem is available and behaving as expected.
    #[default]
    Healthy,
    /// The represented subsystem is unavailable, degraded, or reporting an error.
    Error,
}

/// Drag/drop overlay content for pointer-following feedback.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DragOverlay {
    /// Whether a drag payload is currently active.
    pub active: bool,
    /// Human-friendly payload label.
    pub label: String,
    /// Current hover target label.
    pub target_label: String,
    /// Whether the current target is a valid drop.
    pub valid_target: bool,
    /// Cursor anchor x-coordinate for the floating drag chip, when available.
    pub pointer_x: Option<u16>,
    /// Cursor anchor y-coordinate for the floating drag chip, when available.
    pub pointer_y: Option<u16>,
}

/// Status for an application update check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    /// No update activity in progress.
    #[default]
    Idle,
    /// Update check is running.
    Checking,
    /// A newer update is available.
    Available,
    /// Update check failed.
    Error,
}

/// Update panel state for application chrome and feedback surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UpdatePanel {
    /// Current update-check status.
    pub status: UpdateStatus,
    /// Status label rendered in application chrome.
    pub status_label: String,
    /// Action hint label rendered near update controls.
    pub action_hint_label: String,
    /// Supplemental release-notes label rendered under update hints.
    pub release_notes_label: String,
    /// Available version label, when present.
    pub available_version_label: Option<String>,
    /// Available release URL, when present.
    pub available_url: Option<String>,
    /// Last error message from update checks, if any.
    pub last_error: Option<String>,
}

/// Generic intent category for host-provided confirmation prompts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptIntent {
    /// Confirm a destructive or irreversible operation.
    DestructiveOperation,
    /// Rename the focused content item.
    RenameContent,
    /// Rename an item in a navigation surface.
    RenameNavigationItem,
    /// Create an item in a navigation surface.
    CreateNavigationItem,
    /// Restore retained items after a recoverable operation.
    RestoreRetainedItems,
    /// Permanently purge retained items after a recoverable operation.
    PurgeRetainedItems,
    /// Edit a configuration value.
    EditConfiguration,
}

/// Modal confirmation prompt content parameterized by host-owned prompt kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfirmPrompt<Kind> {
    /// Whether the prompt is currently visible.
    pub visible: bool,
    /// Host-owned prompt kind used to resolve confirm/cancel behavior.
    pub kind: Option<Kind>,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
    /// Confirm action label.
    pub confirm_label: String,
    /// Cancel action label.
    pub cancel_label: String,
    /// Optional target label shown as supplemental metadata.
    pub target_label: Option<String>,
    /// Optional editable prompt input value.
    pub input_value: Option<String>,
    /// Placeholder text for editable prompt input fields.
    pub input_placeholder: Option<String>,
    /// Optional validation error shown below editable prompt input.
    pub input_error: Option<String>,
}

impl<Kind> Default for ConfirmPrompt<Kind> {
    fn default() -> Self {
        Self {
            visible: false,
            kind: None,
            title: String::new(),
            message: String::new(),
            confirm_label: String::new(),
            cancel_label: String::new(),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConfirmPrompt, DragOverlay, HealthState, PromptIntent, RecoverySummary, StatusLineEntry,
        StatusLineLog, UpdatePanel, UpdateStatus,
    };

    #[test]
    fn status_line_log_keeps_latest_bounded_message() {
        let mut log = StatusLineLog::new(3);

        log.publish("button", "pressed");
        log.publish("worker", "started");
        log.publish("animation", "stopped");

        assert_eq!(log.len(), 3);
        assert_eq!(log.latest(), "animation: stopped");
        assert_eq!(
            log.recent_lines(),
            vec![
                "animation: stopped".to_string(),
                "worker: started".to_string(),
                "button: pressed".to_string()
            ]
        );
    }

    #[test]
    fn status_line_entry_exposes_source_message_and_line() {
        let entry = StatusLineEntry::new("worker", "finished");

        assert_eq!(entry.source(), "worker");
        assert_eq!(entry.message(), "finished");
        assert_eq!(entry.line(), "worker: finished");
    }

    #[test]
    fn recovery_summary_defaults_to_idle_and_empty() {
        let summary = RecoverySummary::default();

        assert!(!summary.in_progress);
        assert_eq!(summary.entry_count, 0);
        assert_eq!(summary.retained_count, 0);
    }

    #[test]
    fn health_state_defaults_to_healthy() {
        assert_eq!(HealthState::default(), HealthState::Healthy);
    }

    #[test]
    fn drag_overlay_defaults_to_inactive_and_unanchored() {
        let overlay = DragOverlay::default();

        assert!(!overlay.active);
        assert_eq!(overlay.label, "");
        assert_eq!(overlay.target_label, "");
        assert!(!overlay.valid_target);
        assert_eq!(overlay.pointer_x, None);
        assert_eq!(overlay.pointer_y, None);
    }

    #[test]
    fn update_panel_defaults_to_idle_without_release_metadata() {
        let panel = UpdatePanel::default();

        assert_eq!(panel.status, UpdateStatus::Idle);
        assert_eq!(panel.status_label, "");
        assert_eq!(panel.action_hint_label, "");
        assert_eq!(panel.release_notes_label, "");
        assert_eq!(panel.available_version_label, None);
        assert_eq!(panel.available_url, None);
        assert_eq!(panel.last_error, None);
    }

    #[test]
    fn confirm_prompt_defaults_to_hidden_without_host_kind() {
        let prompt = ConfirmPrompt::<u8>::default();

        assert!(!prompt.visible);
        assert_eq!(prompt.kind, None);
        assert_eq!(prompt.title, "");
        assert_eq!(prompt.message, "");
        assert_eq!(prompt.confirm_label, "");
        assert_eq!(prompt.cancel_label, "");
        assert_eq!(prompt.target_label, None);
        assert_eq!(prompt.input_value, None);
        assert_eq!(prompt.input_placeholder, None);
        assert_eq!(prompt.input_error, None);
    }

    #[test]
    fn prompt_intent_exposes_generic_confirmation_categories() {
        let intents = [
            PromptIntent::DestructiveOperation,
            PromptIntent::RenameContent,
            PromptIntent::RenameNavigationItem,
            PromptIntent::CreateNavigationItem,
            PromptIntent::RestoreRetainedItems,
            PromptIntent::PurgeRetainedItems,
            PromptIntent::EditConfiguration,
        ];

        assert_eq!(intents.len(), 7);
    }
}
