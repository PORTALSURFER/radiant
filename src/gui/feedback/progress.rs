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

#[cfg(test)]
mod tests {
    use super::{
        ProgressOverlay, horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect,
        horizontal_progress_activity_rect, horizontal_progress_fill_rect,
        horizontal_progress_track_rect,
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
}
