use super::super::super::{
    VirtualListController, VirtualListFocusTarget, VirtualListFollowState, VirtualListProjection,
};
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
fn virtual_list_controller_configures_and_focuses_in_one_projection_step() {
    let mut controller = VirtualListController::new();
    controller.set_viewport_start(90);

    let window = controller.configure_and_focus_optional(100, 10, 2, 2, Some(4));

    assert_eq!(window.viewport_start, 0);
    assert_eq!(window.window_start, 0);
    assert_eq!(window.window_end, 12);
    assert_eq!(controller.total_items(), 100);
    assert_eq!(controller.viewport_len(), 10);
    assert_eq!(controller.overscan(), 2);
    assert_eq!(controller.guard_band(), 2);
    assert_eq!(controller.focused_index(), Some(4));
}

#[test]
fn virtual_list_projection_names_projection_inputs() {
    let mut controller = VirtualListController::new();
    let projection = VirtualListProjection::new(100, 10, 2, 2).with_context_row();

    let window = controller.configure_projection_and_focus_optional(projection, Some(8));

    assert_eq!(projection.total_items(), 100);
    assert_eq!(projection.viewport_len(), 10);
    assert_eq!(projection.overscan(), 2);
    assert_eq!(projection.guard_band(), 3);
    assert_eq!(window.viewport_start, 2);
    assert_eq!(controller.total_items(), 100);
    assert_eq!(controller.viewport_len(), 10);
    assert_eq!(controller.overscan(), 2);
    assert_eq!(controller.guard_band(), 3);
    assert_eq!(controller.focused_index(), Some(8));
}

#[test]
fn virtual_list_controller_follows_changed_focus_without_overriding_manual_scroll() {
    let mut controller = VirtualListController::new();
    let mut follow = VirtualListFollowState::new();

    let first = controller.configure_and_focus_changed_optional_with_context_row(
        &mut follow,
        100,
        10,
        2,
        2,
        VirtualListFocusTarget::new(Some("sample-40"), Some(40)),
    );
    assert_eq!(first.viewport_start, 34);
    assert_eq!(follow.focus_key(), Some(&"sample-40"));

    controller.set_scroll_offset(80.0 * 22.0, 22.0);
    let stable = controller.configure_and_focus_changed_optional_with_context_row(
        &mut follow,
        100,
        10,
        2,
        2,
        VirtualListFocusTarget::new(Some("sample-40"), Some(40)),
    );

    assert_eq!(stable.viewport_start, 80);
    assert_eq!(controller.focused_index(), None);

    let changed = controller.configure_and_focus_changed_optional_with_context_row(
        &mut follow,
        100,
        10,
        2,
        2,
        VirtualListFocusTarget::new(Some("sample-05"), Some(5)),
    );

    assert_eq!(changed.viewport_start, 2);
    assert_eq!(follow.focus_key(), Some(&"sample-05"));
}

#[test]
fn virtual_list_projection_follows_changed_focus() {
    let mut controller = VirtualListController::new();
    let mut follow = VirtualListFollowState::new();
    let projection = VirtualListProjection::new(80, 8, 2, 1).with_context_rows(2);

    let first = controller.configure_projection_and_focus_changed_optional(
        &mut follow,
        projection,
        VirtualListFocusTarget::new(Some("row-30"), Some(30)),
    );
    assert_eq!(first.viewport_start, 26);

    controller.set_scroll_offset(50.0 * 22.0, 22.0);
    let stable = controller.configure_projection_and_focus_changed_optional(
        &mut follow,
        projection,
        VirtualListFocusTarget::new(Some("row-30"), Some(30)),
    );
    assert_eq!(stable.viewport_start, 50);

    let changed = controller.configure_projection_and_focus_changed_optional(
        &mut follow,
        projection,
        VirtualListFocusTarget::new(Some("row-4"), Some(4)),
    );
    assert_eq!(changed.viewport_start, 1);
}

#[test]
fn virtual_list_controller_clears_focus_when_changed_key_has_no_index() {
    let mut controller = VirtualListController::new();
    let mut follow = VirtualListFollowState::new();

    controller.configure_and_focus_changed_optional(
        &mut follow,
        20,
        6,
        1,
        1,
        VirtualListFocusTarget::new(Some("folder-10"), Some(10)),
    );

    let cleared = controller.configure_and_focus_changed_optional(
        &mut follow,
        20,
        6,
        1,
        1,
        VirtualListFocusTarget::none(),
    );

    assert_eq!(cleared.viewport_start, 6);
    assert_eq!(controller.focused_index(), None);
    assert_eq!(follow.focus_key(), None);
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
