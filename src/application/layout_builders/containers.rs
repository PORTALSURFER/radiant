//! Row, column, grid, and stack layout builders.

use super::collection::collect_children;
use crate::application::{ViewNode, ViewNodeKind, empty};

/// Default main-axis gap for Radiant application row containers.
pub const DEFAULT_ROW_SPACING: f32 = 4.0;

/// Default main-axis gap for Radiant application column containers.
pub const DEFAULT_COLUMN_SPACING: f32 = 4.0;

/// Default gap for Radiant application grid containers.
pub const DEFAULT_GRID_GAP: f32 = 4.0;

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Row {
        spacing: DEFAULT_ROW_SPACING,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed row container with fill-slot children.
pub fn row_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row(children).key(key)
}

/// Build a column container with fill-slot children.
pub fn column<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Column {
        spacing: DEFAULT_COLUMN_SPACING,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed column container with fill-slot children.
pub fn column_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    column(children).key(key)
}

/// Build a grid container with a fixed column count and default gaps.
pub fn grid<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
    columns: usize,
) -> ViewNode<Message> {
    grid_with_gaps(children, columns, DEFAULT_GRID_GAP, DEFAULT_GRID_GAP)
}

/// Build a grid container with a fixed column count and explicit gaps.
pub fn grid_with_gaps<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
    columns: usize,
    column_gap: f32,
    row_gap: f32,
) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Grid {
        columns,
        column_gap,
        row_gap,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a wrapping flow container with explicit item and line gaps.
pub fn wrap<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
    item_gap: f32,
    line_gap: f32,
) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Wrap {
        item_gap,
        line_gap,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a stack container that overlays children in paint order.
pub fn stack<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Stack { children })
        .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build an overlay stack only when multiple layers are present.
///
/// This is useful for base content with optional overlays: zero layers lower to
/// [`empty`], one layer is returned unchanged, and multiple layers lower to a
/// normal [`stack`].
pub fn stack_layers<Message: 'static>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    let (mut children, has_reserved_descendant_identity) = collect_children(children);
    match children.len() {
        0 => empty(),
        1 => children.remove(0),
        _ => ViewNode::new(ViewNodeKind::Stack { children })
            .with_reserved_descendant_identity(has_reserved_descendant_identity),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, stack_layers, text},
        layout::{ContainerKind, LayoutNode, Vector2},
    };

    #[test]
    fn stack_layers_without_children_lowers_to_empty_widget() {
        let layout = stack_layers::<()>([]).into_surface().layout_node();

        let LayoutNode::Widget(widget) = layout else {
            panic!("empty layer stack should lower to a widget leaf");
        };
        assert_eq!(widget.intrinsic, Vector2::new(0.0, 0.0));
    }

    #[test]
    fn stack_layers_with_one_child_returns_child_without_stack_container() {
        let layout = stack_layers([text::<()>("Only")])
            .into_surface()
            .layout_node();

        assert!(
            matches!(layout, LayoutNode::Widget(_)),
            "single layer should not allocate a stack container"
        );
    }

    #[test]
    fn stack_layers_with_multiple_children_lowers_to_stack_container() {
        let layout = stack_layers([text::<()>("Base"), text::<()>("Overlay")])
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("multiple layers should lower to a container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Stack);
        assert_eq!(container.children.len(), 2);
    }
}
