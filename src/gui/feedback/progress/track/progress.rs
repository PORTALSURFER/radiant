use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

#[cfg(test)]
#[path = "progress/tests.rs"]
mod tests;

/// Return the filled leading segment for a horizontal progress track.
///
/// The returned rect is clamped to `track` and omitted when either the track or
/// the clamped progress fraction has no visible area.
pub fn horizontal_progress_fill_rect(track: Rect, progress_fraction: f32) -> Option<Rect> {
    if !track.has_finite_positive_area() {
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
    if !track.has_finite_positive_area() {
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
