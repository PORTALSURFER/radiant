use super::*;
use crate::{
    application::{ViewNode, button},
    gui::{
        list::{TreeGuideRow, TreeGuideStyle, VirtualListWindow},
        types::Rgba8,
    },
    layout::NodeId,
};

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
