//! Floating overlay layout builders.

use crate::application::{ViewNode, ViewNodeKind, button, primary_style};
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

/// Build a full-size transparent layer that emits a dismiss message when activated.
///
/// Use this behind popovers, menus, dropdowns, and transient panels that should
/// close when the user clicks outside the foreground content.
pub fn dismiss_layer<Message>(message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button("")
        .message(message)
        .key("dismiss-layer")
        .input_only()
        .fill()
}

/// Build a non-interactive floating child tree positioned relative to its parent.
///
/// The layer paints regular view content without contributing intrinsic size and
/// does not register its child widgets for pointer or wheel input.
pub fn floating_layer<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
) -> ViewNode<Message> {
    floating_layer_with_input(offset, size, child, false)
}

/// Build a floating child tree positioned relative to its parent.
///
/// Set `interactive` when the floating content should receive pointer, wheel,
/// focus, and state synchronization traversal like normal content.
pub fn floating_layer_with_input<Message>(
    offset: Point,
    size: Vector2,
    child: ViewNode<Message>,
    interactive: bool,
) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::FloatingLayer {
        offset,
        size,
        child: Box::new(child),
        interactive,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
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
