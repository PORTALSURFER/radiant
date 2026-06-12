use crate::application::{ViewNode, ViewNodeKind};
use crate::gui::types::{Point, Rect};
use crate::layout::Vector2;

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: Rect::from_min_size(Point::new(x, y), Vector2::new(width, height)),
        label: Some(label.into()),
    })
}
