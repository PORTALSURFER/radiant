use crate::gui::types::{Point, Rect, Vector2};

/// Rectangles for a normalized vertical value marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalValueMarker {
    /// Bottom-anchored value stem.
    pub stem: Rect,
    /// Centered interactive marker handle.
    pub handle: Rect,
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn normalized_fraction(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Return bottom-anchored stem and handle geometry for a normalized vertical value marker.
pub fn vertical_value_marker(
    lane: Rect,
    x: f32,
    value: f32,
    stem_width: f32,
    handle_size: f32,
) -> Option<VerticalValueMarker> {
    if !lane.has_finite_positive_area() || !x.is_finite() {
        return None;
    }
    let stem_width = finite_non_negative(stem_width);
    let handle_size = finite_non_negative(handle_size);
    if stem_width <= 0.0 || handle_size <= 0.0 {
        return None;
    }
    let y = lane.y_for_ratio_from_bottom(normalized_fraction(value));
    Some(VerticalValueMarker {
        stem: Rect::from_min_max(
            Point::new(x - stem_width * 0.5, y),
            Point::new(x + stem_width * 0.5, lane.max.y),
        ),
        handle: Rect::from_min_size(
            Point::new(x - handle_size * 0.5, y - handle_size * 0.5),
            Vector2::new(handle_size, handle_size),
        ),
    })
}
