use super::*;

#[test]
fn gui_list_behavior_tests_stay_grouped_by_list_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/list/tests.rs"))
        .expect("gui list test root should be readable");
    let editable = fs::read_to_string(manifest_dir.join("src/gui/list/tests/editable.rs"))
        .expect("gui list editable tests should be readable");
    let selection = fs::read_to_string(manifest_dir.join("src/gui/list/tests/selection.rs"))
        .expect("gui list selection tests should be readable");
    let virtual_list = fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list.rs"))
        .expect("gui list virtual-list tests should be readable");
    let virtual_window =
        fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list/window.rs"))
            .expect("gui list virtual-list window tests should be readable");
    let virtual_controller =
        fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list/controller.rs"))
            .expect("gui list virtual-list controller tests should be readable");
    let virtual_geometry =
        fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list/geometry.rs"))
            .expect("gui list virtual-list geometry tests should be readable");
    let virtual_scrollbar =
        fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list/scrollbar.rs"))
            .expect("gui list virtual-list scrollbar tests should be readable");
    let virtual_invalidation =
        fs::read_to_string(manifest_dir.join("src/gui/list/tests/virtual_list/invalidation.rs"))
            .expect("gui list virtual-list invalidation tests should be readable");
    let grid = fs::read_to_string(manifest_dir.join("src/gui/list/tests/grid.rs"))
        .expect("gui list virtual-grid tests should be readable");

    assert!(
        root.contains("mod editable;")
            && root.contains("mod selection;")
            && root.contains("mod virtual_list;")
            && root.contains("mod grid;")
            && !root.contains("fn virtual_list_window_clamps_requested_bounds")
            && !root.contains("fn virtual_grid_window_clamps_rows"),
        "gui list test root should index focused behavior groups instead of owning all list cases"
    );
    assert!(
        editable.contains("fn editable_tree_row_preserves_existing_and_draft_state")
            && selection.contains("fn list_selection_controller_tracks_single_toggle")
            && virtual_list.contains("mod window;")
            && virtual_list.contains("mod controller;")
            && virtual_list.contains("mod geometry;")
            && virtual_list.contains("mod scrollbar;")
            && virtual_list.contains("mod invalidation;")
            && !virtual_list.contains("fn virtual_list_scrollbar_rejects_nonfinite_track_geometry")
            && grid.contains("fn virtual_grid_window_handles_empty_zero_column"),
        "gui list behavior tests should stay grouped by editable, selection, virtual-list, and grid concerns"
    );
    assert!(
        virtual_window.contains("fn virtual_list_window_scrolls_when_focus_reaches_guard_band")
            && virtual_controller
                .contains("fn virtual_list_controller_maps_scrollbar_drag_to_viewport_start")
            && virtual_geometry
                .contains("fn virtual_list_hit_testing_returns_stable_logical_indices")
            && virtual_scrollbar
                .contains("fn virtual_list_scrollbar_rejects_nonfinite_track_geometry")
            && virtual_invalidation
                .contains("fn virtual_list_item_state_and_invalidation_are_overlay_oriented"),
        "virtual-list behavior tests should stay split by window, controller, geometry, scrollbar, and invalidation concerns"
    );
}
