use super::*;
use crate::{
    application::{ViewNode, button, column},
    gui::list::VirtualListWindow,
    layout::{ContainerKind, LayoutNode, NodeId},
};

#[test]
fn virtual_list_window_projects_only_materialized_range() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };
    let mut projected = Vec::new();

    let view: ViewNode<()> = virtual_list_window(
        window,
        32.0,
        |index| {
            projected.push(index);
            list_row_id(
                10_000 + index as NodeId,
                [button(format!("Row {index:05}"))
                    .message(())
                    .id(20_000 + index as NodeId)],
            )
        },
        64.0,
    );

    assert_eq!(projected, (116..132).collect::<Vec<_>>());
    let layout = view.into_surface().layout_node();
    assert!(
        count_layout_nodes(&layout) < 64,
        "windowed projection should stay bounded to materialized rows"
    );
}

#[test]
fn virtual_list_window_body_keeps_spacers_generic() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };
    let mut projected_window = None;

    let view: ViewNode<()> = virtual_list_window_body(
        window,
        32.0,
        |window| {
            projected_window = Some(window);
            column((window.window_start..window.window_end).map(|index| {
                list_row_id(
                    30_000 + index as NodeId,
                    [button(format!("Row {index:05}"))
                        .message(())
                        .id(40_000 + index as NodeId)],
                )
            }))
        },
        64.0,
    );

    assert_eq!(projected_window, Some(window));
    let layout = view.into_surface().layout_node();
    assert!(
        count_layout_nodes(&layout) < 68,
        "body projection should stay bounded to the materialized range"
    );
}

#[test]
fn virtual_list_window_body_keeps_body_identity_when_top_spacer_appears() {
    let top_window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 0,
        viewport_end: 8,
        window_start: 0,
        window_end: 12,
    };
    let scrolled_window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };

    let top_body_id = virtual_list_body_container_id(top_window);
    let scrolled_body_id = virtual_list_body_container_id(scrolled_window);

    assert_eq!(
        top_body_id, scrolled_body_id,
        "the materialized body needs stable identity as spacer rows appear and disappear"
    );
}

#[test]
fn virtual_list_window_body_uses_app_owned_spacer_scroll() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };

    let view: ViewNode<()> =
        virtual_list_window_body(window, 32.0, |_| column(Vec::<ViewNode<()>>::new()), 96.0);
    let layout = view.into_surface().layout_node();
    let scroll = first_scroll_container(&layout)
        .expect("virtual-list body should lower to a scroll container");

    assert_eq!(scroll.policy.kind, ContainerKind::ScrollView);
    assert!(
        scroll.policy.virtualization.is_none(),
        "app-owned virtual windows must not let layout-level virtualization cull the body spacer"
    );
}

fn virtual_list_body_container_id(window: VirtualListWindow) -> NodeId {
    let view: ViewNode<()> = virtual_list_window_body(
        window,
        32.0,
        |window| {
            column((window.window_start..window.window_end).map(|index| {
                list_row_id(
                    30_000 + index as NodeId,
                    [button(format!("Row {index:05}"))
                        .message(())
                        .id(40_000 + index as NodeId)],
                )
            }))
        },
        64.0,
    );
    let layout = view.into_surface().layout_node();
    let LayoutNode::Container(scroll) = layout else {
        panic!("virtual-list body should lower to a scroll container");
    };
    let content = scroll
        .children
        .first()
        .expect("scroll container should have content");
    let LayoutNode::Container(content_column) = &content.child else {
        panic!("scroll content should be a column");
    };
    let body = content_column
        .children
        .iter()
        .find_map(|child| match &child.child {
            LayoutNode::Container(container) if container.policy.kind == ContainerKind::Column => {
                Some(container.id)
            }
            _ => None,
        })
        .expect("scroll content should include the materialized body column");
    body
}

fn first_scroll_container(node: &LayoutNode) -> Option<&crate::layout::ContainerNode> {
    match node {
        LayoutNode::Widget(_) => None,
        LayoutNode::Container(container) => {
            if container.policy.kind == ContainerKind::ScrollView {
                return Some(container);
            }
            container
                .children
                .iter()
                .find_map(|child| first_scroll_container(&child.child))
        }
    }
}
