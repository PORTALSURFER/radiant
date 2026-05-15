use crate::gui::types::{Point, Rect};

/// Return the filled leading segment for a horizontal progress track.
///
/// The returned rect is clamped to `track` and omitted when either the track or
/// the clamped progress fraction has no visible area.
pub fn horizontal_progress_fill_rect(track: Rect, progress_fraction: f32) -> Option<Rect> {
    if !track_has_finite_positive_size(track) {
        return None;
    }
    let width = track.width() * normalized_fraction(progress_fraction);
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
    if !track_has_finite_positive_size(track) {
        return None;
    }
    let preferred_width = track.width() * normalized_fraction(segment_fraction);
    let segment_width = preferred_width.clamp(
        finite_nonnegative(min_segment_width).min(track.width()),
        track.width(),
    );
    if segment_width <= 0.0 {
        return None;
    }
    let travel = (track.width() - segment_width).max(0.0);
    let min_x = track.min.x + (travel * normalized_fraction(position_fraction));
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
    if !track_has_finite_positive_size(track) {
        return None;
    }
    let level = normalized_fraction(level_fraction);
    let min_visible_width = finite_nonnegative(min_visible_width);
    if level <= 0.0 && min_visible_width <= 0.0 {
        return None;
    }
    let fill_width =
        (track.width() * level).clamp(min_visible_width.min(track.width()), track.width());
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
    if !track_has_finite_positive_size(track) || value == 0 || max_value == 0 {
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

fn track_has_finite_positive_size(track: Rect) -> bool {
    track.min.x.is_finite()
        && track.min.y.is_finite()
        && track.max.x.is_finite()
        && track.max.y.is_finite()
        && track.width() > 0.0
        && track.height() > 0.0
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
