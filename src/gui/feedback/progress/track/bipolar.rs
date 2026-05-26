use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

#[cfg(test)]
#[path = "bipolar/tests.rs"]
mod tests;

/// Return a normalized bipolar value for a point in a vertical control track.
///
/// The bottom edge maps to `-1.0`, the center maps to `0.0`, and the top edge
/// maps to `1.0`.
pub fn vertical_bipolar_value_at_point(track: Rect, point: Point) -> f32 {
    track.ratio_for_y_from_bottom(point.y) * 2.0 - 1.0
}

/// Return the centered fill rect for a vertical bipolar value track.
///
/// `horizontal_inset` reserves leading/trailing padding. `max_half_height_fraction`
/// controls how much of the track each polarity can fill; use `0.5` for the
/// full half-track or a smaller value to leave top/bottom breathing room.
pub fn vertical_bipolar_fill_rect(
    track: Rect,
    value: f32,
    horizontal_inset: f32,
    max_half_height_fraction: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() || !value.is_finite() {
        return None;
    }
    let magnitude = value.abs().clamp(0.0, 1.0);
    if magnitude <= 0.0 {
        return None;
    }
    let inset = finite_nonnegative(horizontal_inset).min(track.width() * 0.5);
    if track.width() - inset * 2.0 <= 0.0 {
        return None;
    }
    let max_half_height = track.height() * normalized_fraction(max_half_height_fraction).min(0.5);
    if max_half_height <= 0.0 {
        return None;
    }

    let center_y = track.center().y;
    let extent = max_half_height * magnitude;
    let (min_y, max_y) = if value >= 0.0 {
        (center_y - extent, center_y)
    } else {
        (center_y, center_y + extent)
    };
    Some(Rect::from_min_max(
        Point::new(track.min.x + inset, min_y),
        Point::new(track.max.x - inset, max_y),
    ))
}
