use crate::gui::types::{Point, Rect, Vector2};

/// Pointer event projected into a canvas-like widget's local coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasPointer {
    /// Pointer position in the widget host's logical coordinate space.
    pub position: Point,
    /// Pointer position relative to the canvas rectangle.
    pub local: Point,
    /// Local position normalized to the canvas rectangle.
    pub normalized: Vector2,
}

pub(super) fn canvas_pointer(bounds: Rect, position: Point) -> Option<CanvasPointer> {
    let width = bounds.width();
    let height = bounds.height();
    if !position.x.is_finite()
        || !position.y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
    {
        return None;
    }
    let local = Point::new(position.x - bounds.min.x, position.y - bounds.min.y);
    Some(CanvasPointer {
        position,
        local,
        normalized: Vector2::new(
            (local.x / width).clamp(0.0, 1.0),
            (local.y / height).clamp(0.0, 1.0),
        ),
    })
}

pub(super) fn point_delta(origin: Point, position: Point) -> Vector2 {
    Vector2::new(position.x - origin.x, position.y - origin.y)
}
