//! Row, column, grid, and stack layout builders.

use super::collection::collect_children;
use crate::{
    application::{ViewNode, ViewNodeKind, empty},
    layout::{ContainerKind, ContainerPolicy, GridPolicy, LayoutPolicy, WrapPolicy},
};
use std::rc::Rc;

/// Default main-axis gap for Radiant application row containers.
pub const DEFAULT_ROW_SPACING: f32 = 4.0;

/// Default main-axis gap for Radiant application column containers.
pub const DEFAULT_COLUMN_SPACING: f32 = 4.0;

/// Default gap for Radiant application grid containers.
pub const DEFAULT_GRID_GAP: f32 = 4.0;

/// Build a container driven by a custom measure/place layout policy.
pub fn layout<Message, Policy: LayoutPolicy>(
    policy: Policy,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::CustomLayout {
        policy: Rc::new(policy),
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: DEFAULT_ROW_SPACING,
            ..ContainerPolicy::default()
        },
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed row container with fill-slot children.
pub fn row_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row(children).key(key.to_string())
}

/// Build a column container with fill-slot children.
pub fn column<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Column,
            spacing: DEFAULT_COLUMN_SPACING,
            ..ContainerPolicy::default()
        },
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed column container with fill-slot children.
pub fn column_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    column(children).key(key.to_string())
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
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Grid,
            grid: GridPolicy {
                columns,
                column_gap,
                row_gap,
            },
            ..ContainerPolicy::default()
        },
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
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: WrapPolicy { item_gap, line_gap },
            ..ContainerPolicy::default()
        },
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a stack container that overlays children in paint order.
pub fn stack<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Container {
        policy: ContainerPolicy {
            kind: ContainerKind::Stack,
            ..ContainerPolicy::default()
        },
        children,
    })
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
        _ => ViewNode::new(ViewNodeKind::Container {
            policy: ContainerPolicy {
                kind: ContainerKind::Stack,
                ..ContainerPolicy::default()
            },
            children,
        })
        .with_reserved_descendant_identity(has_reserved_descendant_identity),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, column, grid_with_gaps, row, stack, stack_layers, text, wrap},
        layout::{ContainerKind, LayoutNode, SizeModeMain, Vector2},
    };

    #[test]
    fn built_in_containers_lower_through_the_shared_surface_container() {
        let cases = [
            (row([text::<()>("row child")]), ContainerKind::Row, 1),
            (
                column([text::<()>("column child")]),
                ContainerKind::Column,
                1,
            ),
            (
                grid_with_gaps([text::<()>("grid child")], 2, 3.0, 5.0),
                ContainerKind::Grid,
                1,
            ),
            (
                wrap([text::<()>("wrap child")], 6.0, 7.0),
                ContainerKind::Wrap,
                1,
            ),
            (stack([text::<()>("stack child")]), ContainerKind::Stack, 1),
        ];

        for (view, kind, child_count) in cases {
            let LayoutNode::Container(container) = view.into_surface().layout_node() else {
                panic!("built-in container should lower to a SurfaceContainer");
            };
            assert_eq!(container.policy.kind, kind);
            assert_eq!(container.children.len(), child_count);
            if kind == ContainerKind::Stack {
                assert!(matches!(
                    container.children[0].slot.size_main,
                    SizeModeMain::Fill(_)
                ));
            }
        }
    }

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
