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

/// Return the visible segment for determinate or indeterminate progress.
///
/// When `total` is zero, the returned segment uses indeterminate activity
/// geometry. Otherwise, `completed / total` resolves the determinate fill.
pub fn horizontal_progress_track_rect(
    track: Rect,
    completed: usize,
    total: usize,
    activity_position_fraction: f32,
    activity_segment_fraction: f32,
    min_activity_segment_width: f32,
) -> Option<Rect> {
    if total == 0 {
        horizontal_progress_activity_rect(
            track,
            activity_position_fraction,
            activity_segment_fraction,
            min_activity_segment_width,
        )
    } else {
        let fraction = (completed as f32 / total as f32).clamp(0.0, 1.0);
        horizontal_progress_fill_rect(track, fraction)
    }
}

/// Return a leading fill rect for a normalized horizontal meter.
///
/// `min_visible_width` can keep non-empty meter values visible on very narrow
/// tracks. Pass `0.0` when zero-width output should be omitted.
pub fn horizontal_meter_fill_rect(
    track: Rect,
    level_fraction: f32,
    min_visible_width: f32,
) -> Option<Rect> {
    if track.width() <= 0.0 || track.height() <= 0.0 {
        return None;
    }
    let level = level_fraction.clamp(0.0, 1.0);
    if level <= 0.0 && min_visible_width <= 0.0 {
        return None;
    }
    let fill_width =
        (track.width() * level).clamp(min_visible_width.max(0.0).min(track.width()), track.width());
    if fill_width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track.min,
        Point::new(track.min.x + fill_width, track.max.y),
    ))
}

/// Return a pixel-rounded leading fill rect for a discrete horizontal meter.
pub fn horizontal_discrete_meter_fill_rect(
    track: Rect,
    value: u32,
    max_value: u32,
) -> Option<Rect> {
    if track.width() <= 0.0 || track.height() <= 0.0 || value == 0 || max_value == 0 {
        return None;
    }
    let ratio = (value.min(max_value) as f32) / (max_value as f32);
    let fill_width = (track.width() * ratio).round().clamp(0.0, track.width());
    if fill_width <= 0.0 {
        return None;
    }
    Some(Rect::from_min_max(
        track.min,
        Point::new(track.min.x + fill_width, track.max.y),
    ))
}

/// Metrics for compact count-based indicators placed after inline text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorMetrics {
    /// Width of each indicator segment.
    pub unit_width: f32,
    /// Height of each indicator segment.
    pub unit_height: f32,
    /// Horizontal gap between adjacent segments.
    pub unit_gap: f32,
    /// Gap between the preceding text and the first indicator segment.
    pub text_gap: f32,
    /// Maximum number of segments to materialize.
    pub max_count: usize,
}

/// Text-relative placement anchor for compact inline indicators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorAnchor {
    /// Bounds available to the text and indicator cluster.
    pub content_rect: Rect,
    /// X origin where the preceding text is rendered.
    pub text_origin_x: f32,
    /// Rendered width of the preceding text.
    pub text_width: f32,
    /// Right edge available to the indicator cluster.
    pub right_limit_x: f32,
}

/// Resolved compact indicator segment rects.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineIndicatorLayout {
    /// Segment rects, ordered from leading to trailing.
    pub rects: [Rect; 8],
    /// Number of materialized rects in `rects`.
    pub count: usize,
}

/// Return the total width reserved for an inline indicator cluster and text gap.
pub fn inline_indicator_reserved_width(count: usize, metrics: InlineIndicatorMetrics) -> f32 {
    let count = count.min(metrics.max_count).min(8);
    if count == 0 {
        return 0.0;
    }
    let unit_width = metrics.unit_width.max(0.0);
    let unit_gap = metrics.unit_gap.max(0.0);
    (count as f32 * unit_width)
        + ((count.saturating_sub(1)) as f32 * unit_gap)
        + metrics.text_gap.max(0.0)
}

/// Place a compact inline indicator cluster after rendered text.
pub fn inline_indicator_layout(
    anchor: InlineIndicatorAnchor,
    count: usize,
    metrics: InlineIndicatorMetrics,
) -> Option<InlineIndicatorLayout> {
    let count = count.min(metrics.max_count).min(8);
    let content_rect = anchor.content_rect;
    if count == 0 || content_rect.width() <= 0.0 || content_rect.height() <= 0.0 {
        return None;
    }
    let unit_height = metrics
        .unit_height
        .max(0.0)
        .min(content_rect.height().max(1.0));
    let unit_width = metrics
        .unit_width
        .max(0.0)
        .min(content_rect.width().max(1.0));
    if unit_width <= 0.0 || unit_height <= 0.0 {
        return None;
    }
    let unit_gap = metrics.unit_gap.max(0.0);
    let total_width = (count as f32 * unit_width) + ((count.saturating_sub(1)) as f32 * unit_gap);
    let ideal_start_x =
        anchor.text_origin_x + anchor.text_width.max(0.0) + metrics.text_gap.max(0.0);
    let right_limit_x = anchor
        .right_limit_x
        .clamp(content_rect.min.x, content_rect.max.x);
    let max_start_x = (right_limit_x - total_width).max(content_rect.min.x);
    let start_x = ideal_start_x.clamp(content_rect.min.x, max_start_x);
    let min_y = content_rect.min.y + ((content_rect.height() - unit_height) * 0.5).floor();
    let max_y = (min_y + unit_height).min(content_rect.max.y);
    let mut rects = [Rect::from_min_max(content_rect.min, content_rect.min); 8];
    for index in 0..count {
        let min_x = start_x + index as f32 * (unit_width + unit_gap);
        rects[index] = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + unit_width).min(content_rect.max.x), max_y),
        );
    }
    Some(InlineIndicatorLayout { rects, count })
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
        ConfirmPrompt, DragOverlay, HealthState, InlineIndicatorAnchor, InlineIndicatorMetrics,
        ProgressOverlay, PromptIntent, RecoverySummary, UpdatePanel, UpdateStatus,
        horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect,
        horizontal_progress_activity_rect, horizontal_progress_fill_rect,
        horizontal_progress_track_rect, inline_indicator_layout, inline_indicator_reserved_width,
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
    fn horizontal_progress_track_rect_switches_between_activity_and_fill() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let activity =
            horizontal_progress_track_rect(track, 0, 0, 0.5, 0.24, 18.0).expect("activity");
        assert_eq!(activity.min, Point::new(48.0, 20.0));
        assert_eq!(activity.max, Point::new(72.0, 28.0));

        let determinate =
            horizontal_progress_track_rect(track, 1, 4, 0.5, 0.24, 18.0).expect("fill");
        assert_eq!(determinate.min, track.min);
        assert_eq!(determinate.max, Point::new(35.0, 28.0));
    }

    #[test]
    fn horizontal_meter_fill_rect_clamps_level_and_minimum_width() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        let minimum = horizontal_meter_fill_rect(track, 0.0, 1.0).expect("minimum meter");
        assert_eq!(minimum.min, track.min);
        assert_eq!(minimum.max, Point::new(11.0, 28.0));

        let overfilled = horizontal_meter_fill_rect(track, 2.0, 1.0).expect("overfilled meter");
        assert_eq!(overfilled, track);

        assert_eq!(horizontal_meter_fill_rect(track, 0.0, 0.0), None);
    }

    #[test]
    fn horizontal_discrete_meter_fill_rect_rounds_and_clamps_byte_levels() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));

        assert_eq!(horizontal_discrete_meter_fill_rect(track, 0, 255), None);
        assert_eq!(horizontal_discrete_meter_fill_rect(track, 1, 255), None);

        let half = horizontal_discrete_meter_fill_rect(track, 128, 255).expect("half meter");
        assert_eq!(half.min, track.min);
        assert_eq!(half.max, Point::new(60.0, 28.0));

        let full = horizontal_discrete_meter_fill_rect(track, 999, 255).expect("full meter");
        assert_eq!(full, track);
    }

    #[test]
    fn inline_indicator_reserved_width_includes_text_gap_and_unit_gaps() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 6.0,
            unit_height: 5.0,
            unit_gap: 2.0,
            text_gap: 4.0,
            max_count: 3,
        };

        assert_eq!(inline_indicator_reserved_width(0, metrics), 0.0);
        assert_eq!(inline_indicator_reserved_width(2, metrics), 18.0);
        assert_eq!(inline_indicator_reserved_width(9, metrics), 26.0);
    }

    #[test]
    fn inline_indicator_layout_places_segments_after_text_and_clamps_to_right_limit() {
        let metrics = InlineIndicatorMetrics {
            unit_width: 6.0,
            unit_height: 5.0,
            unit_gap: 2.0,
            text_gap: 4.0,
            max_count: 3,
        };
        let anchor = InlineIndicatorAnchor {
            content_rect: Rect::from_min_max(Point::new(10.0, 20.0), Point::new(60.0, 30.0)),
            text_origin_x: 16.0,
            text_width: 14.0,
            right_limit_x: 44.0,
        };

        let layout = inline_indicator_layout(anchor, 3, metrics).expect("indicator layout");

        assert_eq!(layout.count, 3);
        assert_eq!(
            &layout.rects[..layout.count],
            &[
                Rect::from_min_max(Point::new(22.0, 22.0), Point::new(28.0, 27.0)),
                Rect::from_min_max(Point::new(30.0, 22.0), Point::new(36.0, 27.0)),
                Rect::from_min_max(Point::new(38.0, 22.0), Point::new(44.0, 27.0)),
            ]
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
