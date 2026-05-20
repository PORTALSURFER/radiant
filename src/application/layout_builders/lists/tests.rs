use super::*;
use crate::{
    application::{IntoView, button},
    layout::{ContainerKind, LayoutNode, NodeId},
};

#[test]
fn virtual_list_uses_packed_rows() {
    let layout = virtual_list(
        0..3,
        |index| button(format!("Row {index}")).message(()).height(22.0),
        44.0,
    )
    .into_surface()
    .layout_node();
    let LayoutNode::Container(scroll) = layout else {
        panic!("virtual list should lower to a scroll container");
    };
    assert_eq!(scroll.policy.kind, ContainerKind::ScrollView);
    let LayoutNode::Container(content) = &scroll.children[0].child else {
        panic!("virtual list scroll content should be a column");
    };
    assert_eq!(content.policy.kind, ContainerKind::Column);
    assert_eq!(content.policy.spacing, 0.0);
}

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

fn count_layout_nodes(node: &LayoutNode) -> usize {
    match node {
        LayoutNode::Widget(_) => 1,
        LayoutNode::Container(container) => {
            1 + container
                .children
                .iter()
                .map(|child| count_layout_nodes(&child.child))
                .sum::<usize>()
        }
    }
}
