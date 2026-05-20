use super::super::{VirtualGridWindow, VirtualGridWindowRequest, resolve_virtual_grid_window};

#[test]
fn virtual_grid_window_clamps_rows_and_applies_overscan() {
    let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
        total_items: 103,
        columns: 5,
        viewport_rows: 4,
        requested_row: 99,
        overscan_rows: 2,
        ..VirtualGridWindowRequest::default()
    });

    assert_eq!(
        window,
        VirtualGridWindow {
            total_items: 103,
            columns: 5,
            total_rows: 21,
            viewport_row_start: 17,
            viewport_row_end: 21,
            window_row_start: 15,
            window_row_end: 21,
            item_start: 75,
            item_end: 103,
        }
    );
    assert_eq!(window.viewport_row_len(), 4);
    assert_eq!(window.window_row_len(), 6);
    assert_eq!(window.item_len(), 28);
    assert!(window.contains_item(102));
    assert!(!window.contains_item(74));
}

#[test]
fn virtual_grid_window_keeps_interior_focus_stable() {
    let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
        total_items: 1_000,
        columns: 4,
        viewport_rows: 10,
        requested_row: 40,
        previous_row: Some(40),
        focused_index: Some(178),
        guard_rows: 2,
        ..VirtualGridWindowRequest::default()
    });

    assert_eq!(window.viewport_row_start, 40);
    assert_eq!(window.viewport_row_end, 50);
}

#[test]
fn virtual_grid_window_scrolls_when_focus_reaches_guard_row() {
    let top = resolve_virtual_grid_window(VirtualGridWindowRequest {
        total_items: 1_000,
        columns: 4,
        viewport_rows: 10,
        requested_row: 40,
        previous_row: Some(40),
        focused_index: Some(164),
        guard_rows: 2,
        ..VirtualGridWindowRequest::default()
    });
    let bottom = resolve_virtual_grid_window(VirtualGridWindowRequest {
        total_items: 1_000,
        columns: 4,
        viewport_rows: 10,
        requested_row: 40,
        previous_row: Some(40),
        focused_index: Some(192),
        guard_rows: 2,
        ..VirtualGridWindowRequest::default()
    });

    assert_eq!(top.viewport_row_start, 39);
    assert_eq!(bottom.viewport_row_start, 41);
}

#[test]
fn virtual_grid_window_handles_empty_zero_column_or_zero_viewport_requests() {
    assert!(resolve_virtual_grid_window(VirtualGridWindowRequest::default()).is_empty());
    assert!(
        resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 10,
            columns: 0,
            viewport_rows: 2,
            ..VirtualGridWindowRequest::default()
        })
        .is_empty()
    );
    assert!(
        resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 10,
            columns: 3,
            viewport_rows: 0,
            ..VirtualGridWindowRequest::default()
        })
        .is_empty()
    );
}
