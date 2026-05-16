use crate::gui::types::{Point, Rect, Vector2};

use super::anchored::anchored_panel_rect;

/// Drag state for a floating panel title bar or handle.
///
/// The state stores the pointer offset inside the panel so a drag can move the
/// panel without snapping its top-left corner to the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatingPanelDrag {
    /// Pointer offset from the panel origin captured when the drag starts.
    pub grab_offset: Vector2,
}

impl FloatingPanelDrag {
    /// Start a floating-panel drag from the current panel rectangle and pointer.
    pub fn new(panel_rect: Rect, pointer: Point) -> Self {
        Self {
            grab_offset: Vector2::new(
                finite_or(pointer.x - panel_rect.min.x, 0.0),
                finite_or(pointer.y - panel_rect.min.y, 0.0),
            ),
        }
    }

    /// Return the requested panel origin for the current pointer position.
    pub fn origin_for_pointer(self, pointer: Point) -> Point {
        Point::new(
            finite_or(pointer.x - self.grab_offset.x, 0.0),
            finite_or(pointer.y - self.grab_offset.y, 0.0),
        )
    }
}

/// Return a floating panel rectangle with its origin clamped inside `bounds`.
///
/// The panel keeps its requested size. If the available bounds are too small,
/// the origin clamps to the inset minimum and the panel may extend past the
/// opposite edge rather than silently shrinking.
pub fn floating_panel_rect(bounds: Rect, origin: Point, size: Vector2, inset: f32) -> Rect {
    anchored_panel_rect(bounds, origin, size, inset)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
