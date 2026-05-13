//! Floating overlay layout builders.

use crate::application::{ViewNode, ViewNodeKind, primary_style};
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
