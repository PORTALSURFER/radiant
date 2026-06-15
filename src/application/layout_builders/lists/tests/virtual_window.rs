use super::*;
use crate::{
    application::{ViewNode, button, column},
    gui::list::VirtualListWindow,
    layout::{ContainerKind, LayoutNode, NodeId, VirtualizationAxis},
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
fn virtual_list_window_body_applies_virtual_scroll_overscan() {
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
    let policy = first_scroll_virtualization_policy(&layout)
        .expect("virtual-list body should lower to a virtualized scroll container");

    assert!(policy.enabled);
    assert_eq!(policy.axis, VirtualizationAxis::Vertical);
    assert_eq!(policy.overscan_px, 96.0);
}

fn first_scroll_virtualization_policy(
    node: &LayoutNode,
) -> Option<crate::layout::VirtualizationPolicy> {
    match node {
        LayoutNode::Widget(_) => None,
        LayoutNode::Container(container) => {
            if container.policy.kind == ContainerKind::ScrollView {
                return container.policy.virtualization;
            }
            container
                .children
                .iter()
                .find_map(|child| first_scroll_virtualization_policy(&child.child))
        }
    }
}
