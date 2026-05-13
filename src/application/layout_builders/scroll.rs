//! Scroll viewport layout builders.

use crate::application::{ViewNode, ViewNodeKind};

/// Build a scroll viewport around one child view.
pub fn scroll<Message>(child: ViewNode<Message>) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::Scroll {
        child: Box::new(child),
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a vertically virtualized scroll viewport around one child view.
pub fn virtual_scroll<Message>(child: ViewNode<Message>, overscan_px: f32) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::VirtualScroll {
        child: Box::new(child),
        overscan_px,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}
