use crate::application::{ViewNode, ViewNodeKind, feedback_overlay, primary_style, row, spacer};
use crate::gui::types::{Point, Rect, Rgba8};
use crate::layout::Vector2;

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: Rect::from_min_size(Point::new(x, y), Vector2::new(width, height)),
        label: None,
    })
    .style(primary_style())
}

/// Build a non-interactive insertion marker positioned in local layout coordinates.
///
/// This is useful inside stack layers where the marker should align to sibling
/// content such as list rows, table headers, or local drop targets rather than
/// using surface coordinates.
pub fn local_drop_marker<Message: 'static>(
    x: f32,
    color: Rgba8,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    row([
        spacer().width(x.max(0.0)),
        feedback_overlay()
            .background(color)
            .view()
            .width(width.max(1.0))
            .height(height.max(1.0)),
        spacer().fill_width(),
    ])
    .spacing(0.0)
}
