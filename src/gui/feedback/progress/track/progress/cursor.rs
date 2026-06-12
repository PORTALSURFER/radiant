use super::super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

/// Return a full-height cursor strip centered on a normalized horizontal value.
///
/// The cursor center is snapped to the nearest logical pixel to keep narrow
/// realtime cursors visually stable while progress or playback values move in
/// sub-pixel increments. `cursor_width` is clamped to the track width and never
/// less than one logical pixel.
pub fn horizontal_value_cursor_rect(
    track: Rect,
    value_fraction: f32,
    cursor_width: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let width = finite_nonnegative(cursor_width)
        .ceil()
        .clamp(1.0, track.width());
    if width <= 0.0 {
        return None;
    }
    let x = track
        .x_for_ratio(normalized_fraction(value_fraction))
        .round()
        .clamp(track.min.x, track.max.x);
    let left = (x - width * 0.5).clamp(track.min.x, (track.max.x - width).max(track.min.x));
    let right = (left + width).min(track.max.x);
    if right <= left {
        return None;
    }
    Some(Rect::from_min_max(
        Point::new(left, track.min.y),
        Point::new(right, track.max.y),
    ))
}
