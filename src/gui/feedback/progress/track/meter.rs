use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect};

#[cfg(test)]
#[path = "meter/tests.rs"]
mod tests;

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
