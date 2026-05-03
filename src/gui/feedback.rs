//! Generic user-feedback surface primitives.

use crate::gui::types::{Point, Rect};

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

/// Return the filled leading segment for a horizontal progress track.
///
/// The returned rect is clamped to `track` and omitted when either the track or
/// the clamped progress fraction has no visible area.
pub fn horizontal_progress_fill_rect(track: Rect, progress_fraction: f32) -> Option<Rect> {
    if track.width() <= 0.0 || track.height() <= 0.0 {
        return None;
    }
    let width = track.width() * progress_fraction.clamp(0.0, 1.0);
    if width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track.min,
        Point::new(track.min.x + width.min(track.width()), track.max.y),
    ))
}

/// Return the moving segment used for an indeterminate horizontal progress track.
///
/// `position_fraction` is the normalized travel position for the segment.
/// `segment_fraction` controls the preferred width relative to the track, and
/// `min_segment_width` keeps the activity segment visible on wider tracks.
pub fn horizontal_progress_activity_rect(
    track: Rect,
    position_fraction: f32,
    segment_fraction: f32,
    min_segment_width: f32,
) -> Option<Rect> {
    if track.width() <= 0.0 || track.height() <= 0.0 {
        return None;
    }
    let preferred_width = track.width() * segment_fraction.clamp(0.0, 1.0);
    let segment_width =
        preferred_width.clamp(min_segment_width.max(0.0).min(track.width()), track.width());
    if segment_width <= 0.0 {
        return None;
    }
    let travel = (track.width() - segment_width).max(0.0);
    let min_x = track.min.x + (travel * position_fraction.clamp(0.0, 1.0));
    Some(Rect::from_min_max(
        Point::new(min_x, track.min.y),
        Point::new((min_x + segment_width).min(track.max.x), track.max.y),
    ))
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
        ConfirmPrompt, DragOverlay, HealthState, ProgressOverlay, PromptIntent, RecoverySummary,
        UpdatePanel, UpdateStatus, horizontal_progress_activity_rect,
        horizontal_progress_fill_rect,
    };
    use crate::gui::types::{Point, Rect};

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
    fn horizontal_progress_fill_rect_clamps_to_track() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let overfilled = horizontal_progress_fill_rect(track, 1.5).expect("filled rect");
        assert_eq!(overfilled.min, track.min);
        assert_eq!(overfilled.max, track.max);

        let partial = horizontal_progress_fill_rect(track, 0.25).expect("partial rect");
        assert_eq!(partial.min, track.min);
        assert_eq!(partial.max, Point::new(35.0, 28.0));
    }

    #[test]
    fn horizontal_progress_fill_rect_omits_empty_tracks_and_zero_fraction() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
        let empty_width = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(10.0, 28.0));
        let empty_height = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 20.0));

        assert_eq!(horizontal_progress_fill_rect(track, 0.0), None);
        assert_eq!(horizontal_progress_fill_rect(track, -0.5), None);
        assert_eq!(horizontal_progress_fill_rect(empty_width, 0.5), None);
        assert_eq!(horizontal_progress_fill_rect(empty_height, 0.5), None);
    }

    #[test]
    fn horizontal_progress_activity_rect_resolves_moving_segment() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let start =
            horizontal_progress_activity_rect(track, 0.0, 0.24, 18.0).expect("start segment");
        assert_eq!(start.min, track.min);
        assert_eq!(start.max, Point::new(34.0, 28.0));

        let end = horizontal_progress_activity_rect(track, 1.0, 0.24, 18.0).expect("end segment");
        assert_eq!(end.min, Point::new(86.0, 20.0));
        assert_eq!(end.max, track.max);
    }

    #[test]
    fn horizontal_progress_activity_rect_clamps_cramped_tracks() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(20.0, 28.0));

        let segment =
            horizontal_progress_activity_rect(track, 0.5, 0.24, 18.0).expect("cramped segment");
        assert_eq!(segment, track);

        assert_eq!(
            horizontal_progress_activity_rect(track, 0.5, 0.0, 0.0),
            None
        );
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
