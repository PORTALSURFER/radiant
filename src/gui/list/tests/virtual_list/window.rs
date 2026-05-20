use super::super::super::{
    VirtualListWindow, VirtualListWindowRequest, resolve_virtual_list_window,
};

#[test]
fn virtual_list_window_clamps_requested_bounds_and_applies_overscan() {
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 100,
        viewport_len: 10,
        requested_start: 95,
        overscan: 3,
        ..VirtualListWindowRequest::default()
    });

    assert_eq!(
        window,
        VirtualListWindow {
            total_items: 100,
            viewport_start: 90,
            viewport_end: 100,
            window_start: 87,
            window_end: 100,
        }
    );
    assert_eq!(window.viewport_len(), 10);
    assert_eq!(window.window_len(), 13);
    assert!(window.contains(99));
    assert!(!window.contains(86));
}

#[test]
fn virtual_list_window_keeps_interior_focus_stable() {
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 1_000,
        viewport_len: 20,
        requested_start: 300,
        previous_start: Some(300),
        focused_index: Some(310),
        guard_band: 4,
        ..VirtualListWindowRequest::default()
    });

    assert_eq!(window.viewport_start, 300);
    assert_eq!(window.viewport_end, 320);
}

#[test]
fn virtual_list_window_scrolls_when_focus_reaches_guard_band() {
    let top = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 1_000,
        viewport_len: 20,
        requested_start: 300,
        previous_start: Some(300),
        focused_index: Some(302),
        guard_band: 4,
        ..VirtualListWindowRequest::default()
    });
    let bottom = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: 1_000,
        viewport_len: 20,
        requested_start: 300,
        previous_start: Some(300),
        focused_index: Some(318),
        guard_band: 4,
        ..VirtualListWindowRequest::default()
    });

    assert_eq!(top.viewport_start, 298);
    assert_eq!(bottom.viewport_start, 303);
}

#[test]
fn virtual_list_window_handles_empty_or_zero_viewport_requests() {
    assert!(resolve_virtual_list_window(VirtualListWindowRequest::default()).is_empty());
    assert!(
        resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 10,
            viewport_len: 0,
            ..VirtualListWindowRequest::default()
        })
        .is_empty()
    );
}
