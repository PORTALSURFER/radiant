//! Floating overlay layout builders.

use crate::application::{ViewNode, ViewNodeKind, primary_style};
use crate::gui::types::Point;
use crate::layout::Vector2;

const DRAG_PREVIEW_OFFSET_X: f32 = 14.0;
const DRAG_PREVIEW_OFFSET_Y: f32 = 18.0;
const DRAG_PREVIEW_DEFAULT_WIDTH: f32 = 168.0;
const DRAG_PREVIEW_DEFAULT_HEIGHT: f32 = 24.0;

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: Some(label.into()),
    })
}

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: None,
    })
    .style(primary_style())
}

/// Build a non-interactive drag preview that follows the pointer.
///
/// The preview is offset from the pointer so it reads like a carried item
/// without covering the exact drop target under the cursor.
pub fn drag_preview<Message>(label: impl Into<String>, pointer: Point) -> ViewNode<Message> {
    drag_preview_sized(
        label,
        pointer,
        Vector2::new(DRAG_PREVIEW_DEFAULT_WIDTH, DRAG_PREVIEW_DEFAULT_HEIGHT),
    )
}

/// Build a non-interactive drag preview with an explicit preview size.
pub fn drag_preview_sized<Message>(
    label: impl Into<String>,
    pointer: Point,
    size: Vector2,
) -> ViewNode<Message> {
    overlay_panel(
        label,
        pointer.x + DRAG_PREVIEW_OFFSET_X,
        pointer.y + DRAG_PREVIEW_OFFSET_Y,
        size.x.max(1.0),
        size.y.max(1.0),
    )
    .style(primary_style())
}
