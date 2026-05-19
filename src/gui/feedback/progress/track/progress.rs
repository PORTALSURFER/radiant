use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

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

#[cfg(test)]
mod tests {
    use super::{
        horizontal_progress_activity_rect, horizontal_progress_fill_rect,
        horizontal_progress_track_rect,
    };
    use crate::gui::types::{Point, Rect};

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
    fn horizontal_progress_fill_rect_rejects_nonfinite_geometry_and_fraction() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
        let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::NAN, 28.0));

        assert_eq!(horizontal_progress_fill_rect(invalid_track, 0.5), None);
        assert_eq!(horizontal_progress_fill_rect(track, f32::NAN), None);
        assert_eq!(horizontal_progress_fill_rect(track, f32::INFINITY), None);
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
    fn horizontal_progress_activity_rect_sanitizes_nonfinite_inputs() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
        let invalid_track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, f32::NAN));

        assert_eq!(
            horizontal_progress_activity_rect(invalid_track, 0.5, 0.24, 18.0),
            None
        );
        assert_eq!(
            horizontal_progress_activity_rect(track, f32::NAN, 0.24, 18.0),
            Some(Rect::from_min_max(
                Point::new(10.0, 20.0),
                Point::new(34.0, 28.0)
            ))
        );
        assert_eq!(
            horizontal_progress_activity_rect(track, 0.5, f32::NAN, 18.0),
            Some(Rect::from_min_max(
                Point::new(51.0, 20.0),
                Point::new(69.0, 28.0)
            ))
        );
        assert_eq!(
            horizontal_progress_activity_rect(track, 0.5, 0.24, f32::NAN),
            Some(Rect::from_min_max(
                Point::new(48.0, 20.0),
                Point::new(72.0, 28.0)
            ))
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
}
