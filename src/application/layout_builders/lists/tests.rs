use super::*;
use crate::{
    application::{IntoView, button, column},
    gui::{
        list::{TreeGuideRow, TreeGuideStyle},
        types::Rgba8,
    },
    layout::{ContainerKind, LayoutNode, NodeId, SizeModeMain},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
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
fn virtual_tree_list_window_projects_rows_and_guide_overlay_together() {
    let window = VirtualListWindow {
        total_items: 10_000,
        viewport_start: 120,
        viewport_end: 128,
        window_start: 116,
        window_end: 132,
    };
    let guides = (0..10_000)
        .map(|index| TreeGuideRow::new(index % 3, index % 4 == 0))
        .collect::<Vec<_>>();
    let style = TreeGuideStyle::new(12.0, 24.0, Rgba8::new(90, 120, 160, 255));
    let mut projected = Vec::new();

    let view: ViewNode<()> = virtual_tree_list_window(
        window,
        24.0,
        &guides,
        style,
        |index| {
            projected.push(index);
            list_row_id(
                50_000 + index as NodeId,
                [button(format!("Folder {index:05}"))
                    .message(())
                    .id(60_000 + index as NodeId)],
            )
        },
        48.0,
    );

    assert_eq!(projected, (116..132).collect::<Vec<_>>());
    let layout = view.into_surface().layout_node();
    assert!(
        count_layout_nodes(&layout) < 72,
        "virtual tree projection should stay bounded to materialized rows plus overlay"
    );
}

#[test]
fn bounded_scroll_column_hides_empty_rows() {
    let view = bounded_scroll_column(Vec::<ViewNode<()>>::new(), 6, 18.0, 6.0);
    let layout = column([view]).into_surface().layout_node();

    let LayoutNode::Container(column) = layout else {
        panic!("parent should lower to a column container");
    };
    assert!(matches!(
        column.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if height == 0.0
    ));
}

#[test]
fn bounded_scroll_column_caps_visible_row_height() {
    let rows = (0..12)
        .map(|index| button(format!("Row {index}")).message(()).height(18.0))
        .collect::<Vec<_>>();
    let view = bounded_scroll_column_from_parts(
        BoundedScrollColumnParts::new(rows, 6, 18.0, 6.0)
            .style(WidgetStyle::new(
                WidgetTone::Neutral,
                WidgetProminence::Subtle,
            ))
            .padding(3.0),
    );

    let layout = column([view]).into_surface().layout_node();
    let LayoutNode::Container(scroll) = layout else {
        panic!("parent should lower to a column container");
    };
    assert!(matches!(
        scroll.children[0].slot.size_main,
        SizeModeMain::Fixed(height) if (height - 114.0).abs() < 0.01
    ));
    let LayoutNode::Container(scroll) = &scroll.children[0].child else {
        panic!("bounded list should lower to a scroll container");
    };
    assert_eq!(scroll.policy.kind, ContainerKind::ScrollView);
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
