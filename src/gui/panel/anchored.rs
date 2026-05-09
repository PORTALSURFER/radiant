use crate::gui::types::{Point, Rect, Vector2};

/// Return a panel rectangle anchored at a point and clamped inside `bounds`.
///
/// The panel keeps its requested size. If the available bounds are too small,
/// the anchor clamps to the inset minimum and the panel may extend past the
/// opposite edge rather than silently shrinking.
pub fn anchored_panel_rect(bounds: Rect, anchor: Point, size: Vector2, inset: f32) -> Rect {
    let inset = inset.max(0.0);
    let width = size.x.max(0.0);
    let height = size.y.max(0.0);
    let min_x = bounds.min.x + inset;
    let max_x = (bounds.max.x - inset - width).max(min_x);
    let min_y = bounds.min.y + inset;
    let max_y = (bounds.max.y - inset - height).max(min_y);
    let panel_min = Point::new(anchor.x.clamp(min_x, max_x), anchor.y.clamp(min_y, max_y));
    Rect::from_min_size(panel_min, Vector2::new(width, height))
}
