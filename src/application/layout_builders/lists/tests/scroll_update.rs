use super::*;
use crate::application::layout_builders::lists::scroll_update::virtual_list_window_needs_materialization;
use crate::{
    gui::{
        list::VirtualListWindow,
        types::{Point, Vector2},
    },
    layout::NodeId,
    runtime::ScrollUpdate,
};

#[test]
fn virtual_list_window_change_for_scroll_maps_offset_to_window() {
    let current = VirtualListWindow {
        total_items: 100,
        viewport_start: 0,
        viewport_end: 10,
        window_start: 0,
        window_end: 12,
    };
    let update = ScrollUpdate {
        node_id: 42 as NodeId,
        position: Point::new(0.0, 0.0),
        delta: Vector2::new(0.0, 220.0),
        previous_offset: Vector2::new(0.0, 0.0),
        offset: Vector2::new(0.0, 220.0),
        viewport: Vector2::new(200.0, 220.0),
    };

    let change = virtual_list_window_change_for_scroll(update, 22.0, current, 2);

    assert_eq!(change.offset_y, 220.0);
    assert_eq!(change.row_height, 22.0);
    assert_eq!(change.window.viewport_start, 10);
    assert_eq!(change.window.viewport_end, 20);
    assert_eq!(change.window.window_start, 8);
    assert_eq!(change.window.window_end, 22);
}

#[test]
fn virtual_list_window_change_uses_runtime_viewport_height() {
    let current = VirtualListWindow {
        total_items: 100,
        viewport_start: 0,
        viewport_end: 80,
        window_start: 0,
        window_end: 84,
    };
    let update = ScrollUpdate {
        node_id: 42 as NodeId,
        position: Point::new(0.0, 0.0),
        delta: Vector2::new(0.0, 880.0),
        previous_offset: Vector2::new(0.0, 0.0),
        offset: Vector2::new(0.0, 880.0),
        viewport: Vector2::new(200.0, 220.0),
    };

    let change = virtual_list_window_change_for_scroll(update, 22.0, current, 2);

    assert_eq!(change.window.viewport_start, 40);
    assert_eq!(change.window.viewport_end, 50);
    assert_eq!(change.window.window_start, 38);
    assert_eq!(change.window.window_end, 52);
}

#[test]
fn virtual_list_window_retains_materialized_rows_until_the_viewport_reaches_an_edge() {
    let current = VirtualListWindow {
        total_items: 100,
        viewport_start: 20,
        viewport_end: 30,
        window_start: 16,
        window_end: 34,
    };

    let inside_overscan = VirtualListWindow {
        viewport_start: 22,
        viewport_end: 32,
        ..current
    };
    let beyond_trailing_edge = VirtualListWindow {
        viewport_start: 25,
        viewport_end: 35,
        ..current
    };
    let resized_viewport = VirtualListWindow {
        viewport_start: 20,
        viewport_end: 31,
        ..current
    };

    assert!(!virtual_list_window_needs_materialization(
        current,
        inside_overscan,
        22.0 * 20.0,
        20.0,
        200.0,
    ));
    assert!(virtual_list_window_needs_materialization(
        current,
        beyond_trailing_edge,
        25.0 * 20.0,
        20.0,
        200.0,
    ));
    assert!(virtual_list_window_needs_materialization(
        current,
        resized_viewport,
        20.0 * 20.0,
        20.0,
        220.0,
    ));
}

#[test]
fn virtual_list_window_retention_covers_a_partially_visible_trailing_row() {
    let current = VirtualListWindow {
        total_items: 100,
        viewport_start: 0,
        viewport_end: 4,
        window_start: 0,
        window_end: 5,
    };
    let next = VirtualListWindow {
        viewport_start: 1,
        viewport_end: 5,
        ..current
    };

    assert!(!virtual_list_window_needs_materialization(
        current, next, 20.0, 20.0, 80.0,
    ));
    assert!(virtual_list_window_needs_materialization(
        current, next, 25.0, 20.0, 80.0,
    ));
}
