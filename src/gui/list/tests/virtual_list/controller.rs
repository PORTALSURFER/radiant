use super::super::super::{
    VirtualListController, VirtualListFocusTarget, VirtualListFollowState, VirtualListProjection,
    VirtualListSliceFocus, VirtualListWindow, VirtualListWindowChange,
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
fn virtual_list_controller_applies_runtime_window_change() {
    let mut controller = VirtualListController::new();
    controller.set_guard_band(2);
    controller.focus(40);

    let window = controller.apply_window_change(VirtualListWindowChange {
        offset_y: 30.0 * 22.0,
        row_height: 22.0,
        window: VirtualListWindow {
            total_items: 100,
            viewport_start: 30,
            viewport_end: 40,
            window_start: 27,
            window_end: 43,
        },
    });

    assert_eq!(window.viewport_start, 30);
    assert_eq!(window.viewport_end, 40);
    assert_eq!(window.window_start, 27);
    assert_eq!(window.window_end, 43);
    assert_eq!(controller.total_items(), 100);
    assert_eq!(controller.viewport_len(), 10);
    assert_eq!(controller.overscan(), 3);
    assert_eq!(controller.guard_band(), 2);
    assert_eq!(controller.focused_index(), None);
}

#[test]
fn virtual_list_controller_checks_projected_viewport_containment() {
    let mut controller = VirtualListController::with_items(100, 10);
    controller.set_viewport_start(95);

    assert!(controller.viewport_contains_index(30, 8, 29));
    assert!(!controller.viewport_contains_index(30, 8, 21));
    assert!(!controller.viewport_contains_index(30, 8, 30));
    assert!(!controller.viewport_contains_index(30, 0, 0));
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
fn virtual_list_focus_target_resolves_key_in_projected_slice() {
    #[derive(Clone)]
    struct Row {
        id: &'static str,
    }

    let rows = [Row { id: "one" }, Row { id: "two" }, Row { id: "three" }];

    let found =
        VirtualListFocusTarget::from_slice_by(&rows, Some(String::from("two")), |row, key| {
            row.id == key.as_str()
        });
    assert_eq!(found.key.as_deref(), Some("two"));
    assert_eq!(found.index, Some(1));

    let missing =
        VirtualListFocusTarget::from_slice_by(&rows, Some(String::from("four")), |row, key| {
            row.id == key.as_str()
        });
    assert_eq!(missing.key, None);
    assert_eq!(missing.index, None);
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
fn virtual_list_controller_follows_changed_focus_from_slice() {
    #[derive(Clone)]
    struct Row {
        id: &'static str,
    }

    let rows: Vec<Row> = (0..60)
        .map(|index| Row {
            id: match index {
                6 => "row-06",
                40 => "row-40",
                _ => "other",
            },
        })
        .collect();
    let mut controller = VirtualListController::new();
    let mut follow = VirtualListFollowState::<String>::new();

    let first = controller.configure_slice_focus_changed_optional(
        &mut follow,
        VirtualListSliceFocus::from_slice_by(
            &rows,
            10,
            2,
            2,
            Some(String::from("row-40")),
            |row, key| row.id == key.as_str(),
        )
        .with_context_row(),
    );
    assert_eq!(first.viewport_start, 34);
    assert_eq!(follow.focus_key().map(String::as_str), Some("row-40"));

    controller.set_scroll_offset(50.0 * 22.0, 22.0);
    let stable = controller.configure_slice_focus_changed_optional(
        &mut follow,
        VirtualListSliceFocus::from_slice_by(
            &rows,
            10,
            2,
            2,
            Some(String::from("row-40")),
            |row, key| row.id == key.as_str(),
        )
        .with_context_row(),
    );
    assert_eq!(stable.viewport_start, 50);

    let changed = controller.configure_slice_focus_changed_optional(
        &mut follow,
        VirtualListSliceFocus::from_slice_by(
            &rows,
            10,
            2,
            2,
            Some(String::from("row-06")),
            |row, key| row.id == key.as_str(),
        ),
    );
    assert_eq!(changed.viewport_start, 4);
    assert_eq!(follow.focus_key().map(String::as_str), Some("row-06"));

    let missing = controller.configure_slice_focus_changed_optional(
        &mut follow,
        VirtualListSliceFocus::from_slice_by(
            &rows,
            10,
            2,
            2,
            Some(String::from("missing")),
            |row, key| row.id == key.as_str(),
        ),
    );
    assert_eq!(missing.viewport_start, 4);
    assert_eq!(follow.focus_key(), None);
    assert_eq!(controller.focused_index(), None);
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
