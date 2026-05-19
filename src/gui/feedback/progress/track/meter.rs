use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

/// Return a leading fill rect for a normalized horizontal meter.
///
/// `min_visible_width` can keep non-empty meter values visible on very narrow
/// tracks. Pass `0.0` when zero-width output should be omitted.
pub fn horizontal_meter_fill_rect(
    track: Rect,
    level_fraction: f32,
    min_visible_width: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
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
    if !track.has_finite_positive_area() || value == 0 || max_value == 0 {
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
    use super::{horizontal_discrete_meter_fill_rect, horizontal_meter_fill_rect};
    use crate::gui::types::{Point, Rect};

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
    fn horizontal_meter_fill_rect_sanitizes_nonfinite_inputs() {
        let track = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 28.0));
        let invalid_track = Rect::from_min_max(Point::new(f32::NAN, 20.0), Point::new(110.0, 28.0));

        assert_eq!(horizontal_meter_fill_rect(invalid_track, 0.5, 1.0), None);
        assert_eq!(horizontal_meter_fill_rect(track, f32::NAN, 0.0), None);
        assert_eq!(
            horizontal_meter_fill_rect(track, f32::NAN, 1.0),
            Some(Rect::from_min_max(
                Point::new(10.0, 20.0),
                Point::new(11.0, 28.0)
            ))
        );
        assert_eq!(
            horizontal_meter_fill_rect(track, 0.5, f32::NAN)
                .unwrap()
                .max,
            Point::new(60.0, 28.0)
        );
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
    fn horizontal_discrete_meter_fill_rect_rejects_nonfinite_tracks() {
        let invalid_track =
            Rect::from_min_max(Point::new(10.0, 20.0), Point::new(f32::INFINITY, 28.0));

        assert_eq!(
            horizontal_discrete_meter_fill_rect(invalid_track, 128, 255),
            None
        );
    }
}
