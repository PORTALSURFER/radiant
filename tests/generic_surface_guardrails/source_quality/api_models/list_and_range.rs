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

#[test]
fn gui_geometry_tests_stay_grouped_by_rect_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests.rs"))
        .expect("gui geometry test root should be readable");
    let bounds =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_bounds.rs"))
            .expect("gui geometry bounds tests should be readable");
    let insets =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_insets.rs"))
            .expect("gui geometry inset tests should be readable");
    let squares =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_squares.rs"))
            .expect("gui geometry square tests should be readable");
    let edges = fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_edges.rs"))
        .expect("gui geometry edge tests should be readable");

    assert!(
        root.contains("mod rect_bounds;")
            && root.contains("mod rect_insets;")
            && root.contains("mod rect_squares;")
            && root.contains("mod rect_edges;")
            && !root.contains("fn rect_centered_pixel_square")
            && !root.contains("fn rect_edge_strips_resolve_each_side"),
        "gui geometry test root should index focused rect behavior groups instead of owning all cases"
    );
    assert!(
        bounds.contains("fn rect_clamp_to_limits_rect_to_bounds")
            && bounds.contains("fn point_and_rect_finiteness_helpers_reject_invalid_geometry")
            && insets.contains("fn rect_inset_uniform_saturating_caps_at_half_extents")
            && squares.contains("fn rect_centered_odd_pixel_square_forces_odd_side")
            && edges.contains("fn rect_edge_strips_resolve_each_side"),
        "gui geometry tests should stay grouped by bounds, insets, centered squares, and edge helpers"
    );
}

#[test]
fn public_layout_policy_models_do_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_dir = manifest_dir.join("src/gui/layout_core/model");
    let mut violations = Vec::new();

    for path in [
        model_dir.join("alignment.rs"),
        model_dir.join("container.rs"),
        model_dir.join("virtualization.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("#[allow(dead_code)]") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "public layout policy models should be exported, tested, or removed instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}

#[test]
fn linear_layout_hot_path_uses_request_objects_instead_of_argument_suppressions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let linear_dir = manifest_dir.join("src/gui/layout_core/engine/layout/linear");
    let mut violations = Vec::new();

    for path in [
        linear_dir.join("placement.rs"),
        linear_dir.join("sizing.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "linear layout measurement and placement should use cohesive request objects instead of suppressing long parameter lists:\n{}",
        violations.join("\n")
    );
}

#[test]
fn editable_tree_rows_use_named_parts_instead_of_boolean_constructor_lists() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct EditableTreeRowParts")
            && source.contains("pub fn from_parts(parts: EditableTreeRowParts) -> Self"),
        "editable tree rows should expose a named parts object for readable public construction"
    );
    assert!(
        !source.contains("too_many_arguments"),
        "editable tree rows should not hide long positional constructors behind clippy suppressions"
    );
}

#[test]
fn column_summaries_use_named_parts_for_title_and_count() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/column.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let editable = fs::read_to_string(manifest_dir.join("src/gui/list/editable.rs"))
        .expect("editable list module should be readable");
    let list = fs::read_to_string(manifest_dir.join("src/gui/list.rs"))
        .expect("list module should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct ColumnSummaryParts")
            && source.contains("pub fn from_parts(parts: ColumnSummaryParts) -> Self"),
        "column summaries should expose named parts for title and item count"
    );
    assert!(
        source.contains("Self::from_parts(ColumnSummaryParts {")
            && editable.contains("ColumnSummaryParts")
            && list.contains("ColumnSummaryParts")
            && lib.contains("ColumnSummaryParts"),
        "column summary compatibility constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn virtual_list_stack_metrics_use_named_parts_for_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/virtual_list/geometry.rs");
    let virtual_list_path = manifest_dir.join("src/gui/list/virtual_list.rs");
    let list_path = manifest_dir.join("src/gui/list.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let virtual_list = fs::read_to_string(&virtual_list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", virtual_list_path.display()));
    let list = fs::read_to_string(&list_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", list_path.display()));
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        source.contains("pub struct VirtualListStackMetricsParts")
            && source.contains("pub fn from_parts(parts: VirtualListStackMetricsParts) -> Self"),
        "virtual-list stack metrics should expose named parts for extent, gap, and viewport cap"
    );
    assert!(
        source.contains("Self::from_parts(VirtualListStackMetricsParts {")
            && virtual_list.contains("VirtualListStackMetricsParts")
            && list.contains("VirtualListStackMetricsParts")
            && lib.contains("VirtualListStackMetricsParts"),
        "virtual-list stack metric constructor and public exports should keep the named-parts path available"
    );
}

#[test]
fn normalized_ranges_use_named_parts_for_milli_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range root should be readable");
    let source_path = manifest_dir.join("src/gui/range/interval.rs");
    let interval = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let interval_tests = fs::read_to_string(manifest_dir.join("src/gui/range/interval/tests.rs"))
        .expect("normalized range behavior tests should be readable");

    assert!(
        root.contains("mod interval;")
            && root.contains("pub use interval::{NormalizedRange, NormalizedRangeParts};")
            && !root.contains("pub struct NormalizedRange"),
        "range root should re-export the normalized interval model without owning its implementation"
    );

    assert!(
        interval.contains("pub struct NormalizedRangeParts")
            && interval.contains("pub fn from_parts(parts: NormalizedRangeParts) -> Self")
            && interval.contains("#[path = \"interval/tests.rs\"]")
            && !interval.contains("fn normalized_range_orders_and_clamps_nano_bounds"),
        "normalized ranges should expose named parts for start and end milli-unit bounds while delegating behavior tests"
    );
    assert!(
        interval_tests.contains("fn normalized_range_orders_and_clamps_nano_bounds")
            && interval_tests.contains("fn normalized_range_supports_named_parts_construction"),
        "normalized range behavior coverage should live in range/interval/tests.rs"
    );
    assert!(
        interval.contains("Self::from_parts(NormalizedRangeParts {"),
        "normalized range compatibility constructor should keep the named-parts path available"
    );
}

#[test]
fn index_viewport_model_keeps_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport.rs"))
        .expect("index viewport model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport/tests.rs"))
        .expect("index viewport behavior tests should be readable");

    assert!(
        range.contains("pub use index_viewport::IndexViewport;")
            && model.contains("pub struct IndexViewport")
            && model.contains("#[path = \"index_viewport/tests.rs\"]")
            && !model.contains("fn index_viewport_clamps_visible_span_and_offset_fraction"),
        "index viewport should stay exported through the range facade while keeping behavior tests out of the model root"
    );
    assert!(
        tests.contains("fn index_viewport_clamps_visible_span_and_offset_fraction")
            && tests.contains("fn index_viewport_zooms_and_pans_around_visible_anchor")
            && tests.contains("fn index_viewport_sets_offset_and_projects_visible_ratio"),
        "index viewport behavior coverage should live in range/index_viewport/tests.rs"
    );
}

#[test]
fn normalized_viewports_use_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/range/viewport.rs");
    let module_path = manifest_dir.join("src/gui/range.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let projection = fs::read_to_string(manifest_dir.join("src/gui/range/viewport/projection.rs"))
        .expect("normalized viewport projection source should be readable");
    let viewport_tests = fs::read_to_string(manifest_dir.join("src/gui/range/viewport/tests.rs"))
        .expect("normalized viewport behavior tests should be readable");
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    assert!(
        source.contains("pub struct NormalizedViewportParts")
            && source.contains("pub fn from_parts(parts: NormalizedViewportParts) -> Self"),
        "normalized viewports should expose named parts for micro and optional nano bounds"
    );
    assert!(
        source.contains("Self::from_parts(NormalizedViewportParts {")
            && source.contains("mod projection;")
            && source.contains("#[path = \"viewport/tests.rs\"]")
            && source.contains("projection::x_for_ratio")
            && !source.contains("fn finite_ordered_x_bounds")
            && !source.contains("fn normalized_viewport_projects_absolute_ratios_into_rect")
            && module.contains("NormalizedViewportParts"),
        "normalized viewport compatibility constructor and range export should keep the named-parts path available while behavior tests stay delegated"
    );
    assert!(
        viewport_tests.contains("fn normalized_viewport_projects_absolute_ratios_into_rect")
            && viewport_tests.contains("fn normalized_viewport_supports_named_parts_construction"),
        "normalized viewport behavior coverage should live in range/viewport/tests.rs"
    );
    assert!(
        projection.contains("fn local_ratio")
            && projection.contains("fn x_for_ratio")
            && projection.contains("fn finite_ordered_x_bounds"),
        "normalized viewport projection math and x-bound sanitization should live in viewport/projection.rs"
    );
}

#[test]
fn normalized_scrollbars_keep_model_and_geometry_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar.rs"))
        .expect("normalized scrollbar root should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/model.rs"))
        .expect("normalized scrollbar model should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/geometry.rs"))
        .expect("normalized scrollbar geometry should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/tests.rs"))
        .expect("normalized scrollbar behavior tests should be readable");
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");

    assert!(
        root.contains("mod geometry;")
            && root.contains("mod model;")
            && root.contains("#[path = \"scrollbar/tests.rs\"]")
            && root.contains("pub use geometry::{")
            && root.contains("pub use model::{NormalizedScrollbar")
            && !root.contains("pub struct NormalizedScrollbarRequest")
            && !root.contains("fn clamped_normalized_span"),
        "normalized scrollbar root should re-export focused model and geometry modules while delegating behavior tests"
    );
    assert!(
        tests.contains("fn normalized_scrollbar_maps_viewport_to_horizontal_thumb")
            && tests.contains("fn normalized_scrollbar_resolves_drag_and_track_click_center"),
        "normalized scrollbar behavior coverage should live in range/scrollbar/tests.rs"
    );
    assert!(
        model.contains("pub struct NormalizedScrollbarRequest")
            && model.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar public DTOs should live in scrollbar/model.rs"
    );
    assert!(
        geometry.contains("pub fn resolve_normalized_scrollbar")
            && geometry.contains("pub fn normalized_scrollbar_center_for_pointer")
            && geometry.contains("fn clamped_normalized_span")
            && !geometry.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar geometry and pointer resolution should live in scrollbar/geometry.rs"
    );
    assert!(
        range.contains("NormalizedScrollbarRequest")
            && range.contains("resolve_normalized_scrollbar")
            && range.contains("normalized_scrollbar_center_at_point"),
        "normalized scrollbar public API should remain available through the range facade"
    );
}
