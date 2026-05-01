//! Generic user-feedback surface primitives.

/// Progress overlay state for long-running operations.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProgressOverlay {
    /// Whether the overlay is currently visible.
    pub visible: bool,
    /// Whether the overlay is modal.
    pub modal: bool,
    /// Title text for the progress surface.
    pub title: String,
    /// Optional detail line.
    pub detail: Option<String>,
    /// Completed steps.
    pub completed: usize,
    /// Total steps.
    pub total: usize,
    /// Whether the running operation supports cancel.
    pub cancelable: bool,
    /// Whether cancel has already been requested.
    pub cancel_requested: bool,
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
    use super::{ConfirmPrompt, DragOverlay, ProgressOverlay, UpdatePanel, UpdateStatus};

    #[test]
    fn progress_overlay_defaults_to_hidden_and_empty() {
        let overlay = ProgressOverlay::default();

        assert!(!overlay.visible);
        assert!(!overlay.modal);
        assert_eq!(overlay.title, "");
        assert_eq!(overlay.detail, None);
        assert_eq!(overlay.completed, 0);
        assert_eq!(overlay.total, 0);
        assert!(!overlay.cancelable);
        assert!(!overlay.cancel_requested);
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
}
