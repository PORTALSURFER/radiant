use super::*;
use crate::{
    application::{ViewNode, button, column},
    gui::list::VirtualListWindow,
    layout::NodeId,
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
