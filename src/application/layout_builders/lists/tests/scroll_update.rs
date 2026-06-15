use super::*;
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
