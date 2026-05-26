use super::sanitize::{finite_nonnegative, normalized_fraction};
use crate::gui::types::{Point, Rect, Vector2};

#[cfg(test)]
#[path = "vertical/tests.rs"]
mod tests;

/// Return a bottom-up normalized value for a point in a vertical track.
pub fn vertical_value_at_point(track: Rect, point: Point) -> f32 {
    track.ratio_for_y_from_bottom(point.y)
}

/// Return a centered vertical rail inside a vertical value track.
pub fn vertical_center_track_rect(track: Rect, width: f32) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let width = finite_nonnegative(width).min(track.width());
    if width <= 0.0 {
        return None;
    }
    let center_x = track.center().x;
    Some(Rect::from_min_max(
        Point::new(center_x - width * 0.5, track.min.y),
        Point::new(center_x + width * 0.5, track.max.y),
    ))
}

/// Return the knob rect for a normalized vertical value track.
pub fn vertical_value_knob_rect(track: Rect, value: f32, knob_height: f32) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let knob_height = finite_nonnegative(knob_height);
    if knob_height <= 0.0 {
        return None;
    }
    let center_y = track.y_for_ratio_from_bottom(normalized_fraction(value));
    Some(Rect::from_min_size(
        Point::new(track.min.x, center_y - knob_height * 0.5),
        Vector2::new(track.width(), knob_height),
    ))
}

/// Return one filled lane rect for a multi-lane vertical meter.
pub fn vertical_meter_lane_fill_rect(
    meter: Rect,
    lane_index: usize,
    lane_count: usize,
    value: f32,
    lane_gap: f32,
    padding: f32,
) -> Option<Rect> {
    if !meter.has_finite_positive_area() || lane_count == 0 {
        return None;
    }
    let value = normalized_fraction(value);
    if value <= 0.0 {
        return None;
    }
    let lane_gap = finite_nonnegative(lane_gap);
    let padding = finite_nonnegative(padding).min(meter.width().min(meter.height()) * 0.5);
    let lane_count = lane_count.max(1);
    let lane_index = lane_index.min(lane_count - 1);
    let inner_width = meter.width() - padding * 2.0;
    let inner_height = meter.height() - padding * 2.0;
    let total_gap = lane_gap * lane_count.saturating_sub(1) as f32;
    let available_width = inner_width - total_gap;
    if inner_width <= 0.0 || inner_height <= 0.0 || available_width <= 0.0 {
        return None;
    }
    let lane_width = available_width / lane_count as f32;
    let x = meter.min.x + padding + lane_index as f32 * (lane_width + lane_gap);
    Some(Rect::from_min_max(
        Point::new(x, meter.max.y - padding - inner_height * value),
        Point::new(x + lane_width, meter.max.y - padding),
    ))
}

/// Return a horizontal marker line for a normalized vertical track value.
pub fn vertical_value_line_rect(
    track: Rect,
    value: f32,
    horizontal_inset: f32,
    height: f32,
) -> Option<Rect> {
    if !track.has_finite_positive_area() {
        return None;
    }
    let inset = finite_nonnegative(horizontal_inset).min(track.width() * 0.5);
    let height = finite_nonnegative(height);
    if track.width() - inset * 2.0 <= 0.0 || height <= 0.0 {
        return None;
    }
    let y = track.y_for_ratio_from_bottom(normalized_fraction(value));
    Some(Rect::from_min_max(
        Point::new(track.min.x + inset, y),
        Point::new(track.max.x - inset, y + height),
    ))
}
