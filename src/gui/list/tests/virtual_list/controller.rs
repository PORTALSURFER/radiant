use super::super::super::VirtualListController;
use crate::gui::types::{Point, Rect};

#[test]
fn virtual_list_controller_preserves_focus_until_guard_band_requires_scroll() {
    let mut controller = VirtualListController::with_items(100, 10);
    controller.set_overscan(2);
    controller.set_guard_band(2);
    controller.set_viewport_start(20);

    let stable = controller.focus(24);
    assert_eq!(stable.viewport_start, 20);
    assert_eq!(stable.window_start, 18);
    assert_eq!(stable.window_end, 32);
    assert_eq!(controller.focused_index(), Some(24));

    let scrolled = controller.focus(29);
    assert_eq!(scrolled.viewport_start, 22);
    assert_eq!(scrolled.viewport_end, 32);
    assert_eq!(controller.viewport_start(), 22);
}

#[test]
fn virtual_list_controller_scrolls_units_and_clamps_after_count_changes() {
    let mut controller = VirtualListController::with_items(30, 8);

    assert_eq!(controller.scroll_units(2.4).unwrap().viewport_start, 2);
    assert_eq!(controller.scroll_rows(100).unwrap().viewport_start, 22);
    controller.set_total_items(12);
    assert_eq!(controller.viewport_start(), 4);
    assert_eq!(controller.resolve().viewport_end, 12);

    controller.set_viewport_len(0);
    assert!(controller.scroll_rows(1).is_none());
    assert!(controller.resolve().is_empty());
}

#[test]
fn virtual_list_controller_maps_scrollbar_drag_to_viewport_start() {
    let mut controller = VirtualListController::with_items(100, 20);
    let track = Rect::from_min_max(Point::new(200.0, 10.0), Point::new(212.0, 210.0));
    let scrollbar = controller.scrollbar(track, 20.0).unwrap();

    let window = controller
        .drag_scrollbar(scrollbar, scrollbar.track.max.y, 0.0)
        .unwrap();
    assert_eq!(window.viewport_start, 80);
    assert_eq!(controller.viewport_start(), 80);
}
