use crate::gui::types::{Point, Rect, Vector2};

/// Return a panel rectangle anchored at a point and clamped inside `bounds`.
///
/// The panel keeps its requested size. If the available bounds are too small,
/// the anchor clamps to the inset minimum and the panel may extend past the
/// opposite edge rather than silently shrinking.
pub fn anchored_panel_rect(bounds: Rect, anchor: Point, size: Vector2, inset: f32) -> Rect {
    let inset = finite_nonnegative(inset);
    let width = finite_nonnegative(size.x);
    let height = finite_nonnegative(size.y);
    let bounds_min_x = finite_or(bounds.min.x, 0.0);
    let bounds_min_y = finite_or(bounds.min.y, 0.0);
    let bounds_max_x = finite_or(bounds.max.x, bounds_min_x).max(bounds_min_x);
    let bounds_max_y = finite_or(bounds.max.y, bounds_min_y).max(bounds_min_y);
    let min_x = bounds_min_x + inset;
    let max_x = (bounds_max_x - inset - width).max(min_x);
    let min_y = bounds_min_y + inset;
    let max_y = (bounds_max_y - inset - height).max(min_y);
    let anchor_x = finite_or(anchor.x, min_x);
    let anchor_y = finite_or(anchor.y, min_y);
    let panel_min = Point::new(anchor_x.clamp(min_x, max_x), anchor_y.clamp(min_y, max_y));
    Rect::from_min_size(panel_min, Vector2::new(width, height))
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
