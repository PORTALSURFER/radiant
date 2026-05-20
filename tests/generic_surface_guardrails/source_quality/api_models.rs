use super::*;

#[test]
fn application_view_lowering_keeps_container_defaults_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/application/view_node.rs"))
        .expect("application view node module should be readable");
    let lowering = fs::read_to_string(manifest_dir.join("src/application/view_node/lowering.rs"))
        .expect("application view lowering should be readable");
    let defaults =
        fs::read_to_string(manifest_dir.join("src/application/view_node/lowering_defaults.rs"))
            .expect("application view lowering defaults should be readable");
    let defaults_tests = fs::read_to_string(
        manifest_dir.join("src/application/view_node/lowering_defaults/tests.rs"),
    )
    .expect("application view lowering default tests should be readable");

    assert!(
        module.contains("mod lowering_defaults;")
            && lowering.contains("ViewNodeContainerDefaults::new("),
        "view lowering should consume container defaults from a focused helper"
    );
    assert!(
        !lowering.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("fn default_container_padding")
            && defaults.contains("fn base_policy")
            && defaults.contains("#[path = \"lowering_defaults/tests.rs\"]")
            && !defaults.contains("fn styled_container_defaults_to_panel_padding"),
        "declarative container default policy should stay outside the main view lowering match"
    );
    assert!(
        defaults_tests.contains("fn styled_container_defaults_to_panel_padding")
            && defaults_tests
                .contains("fn explicit_container_defaults_override_style_padding_and_alignment"),
        "view lowering default behavior tests should live in lowering_defaults/tests.rs"
    );
}

#[test]
fn application_view_identity_keeps_reserved_id_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let identity = fs::read_to_string(manifest_dir.join("src/application/view_node/identity.rs"))
        .expect("application view node identity module should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/view_node/identity/tests.rs"))
            .expect("application view node identity tests should be readable");

    assert!(
        identity.contains("pub(super) fn collect_reserved_ids")
            && identity.contains("fn reserve_child_identity_capacity")
            && identity.contains("fn reserved_identity_capacity_hint")
            && identity.contains("#[path = \"identity/tests.rs\"]")
            && !identity.contains("fn reserved_id_collection_presizes_for_large_child_groups"),
        "view-node identity collection should live in view_node/identity.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn reserved_id_collection_presizes_for_large_child_groups")
            && tests.contains("fn reserved_id_collection_includes_grid_child_identities")
            && tests.contains("fn reserved_id_collection_presizes_wrapped_runtime_identities"),
        "view-node identity behavior coverage should live in view_node/identity/tests.rs"
    );
}

#[test]
fn application_list_builders_keep_virtualization_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lists = fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists.rs"))
        .expect("application list builders should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/layout_builders/lists/tests.rs"))
            .expect("application list builder tests should be readable");

    assert!(
        lists.contains("pub fn virtual_list<Message, Item>")
            && lists.contains("pub fn virtual_list_window<Message: 'static>")
            && lists.contains("fn apply_list_row_chrome<Message>")
            && lists.contains("#[path = \"lists/tests.rs\"]")
            && !lists.contains("fn virtual_list_window_projects_only_materialized_range"),
        "application list builders should own builder logic while virtualization behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn virtual_list_uses_packed_rows")
            && tests.contains("fn virtual_list_window_projects_only_materialized_range")
            && tests.contains("fn count_layout_nodes"),
        "application list builder behavior coverage should live in layout_builders/lists/tests.rs"
    );
}

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

#[test]
fn preference_panel_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/form.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/form/tests.rs"))
        .expect("form root behavior tests should be readable");

    assert!(
        source.contains("pub struct PreferencePanelParts")
            && source.contains("pub fn from_parts(parts: PreferencePanelParts<TOGGLES>) -> Self")
            && source.contains("#[path = \"form/tests.rs\"]")
            && !source.contains(
                "fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label"
            ),
        "preference panel state should expose a named parts object for readable public construction while delegating behavior tests"
    );
    assert!(
        tests.contains("fn option_item_preserves_label_selection_and_value")
            && tests.contains(
                "fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label"
            ),
        "form root behavior coverage should live in form/tests.rs"
    );
    assert!(
        source.contains("mod numeric;")
            && source.contains("mod paired;")
            && source.contains("DecimalTextInputPolicy")
            && source.contains("PairedStatusPanel"),
        "form root should remain the focused public facade for generic form helpers"
    );
    assert!(
        source.contains("Self::from_parts(PreferencePanelParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
}

#[test]
fn form_numeric_and_paired_helpers_keep_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let numeric = fs::read_to_string(manifest_dir.join("src/gui/form/numeric.rs"))
        .expect("form numeric helpers should be readable");
    let numeric_tests = fs::read_to_string(manifest_dir.join("src/gui/form/numeric/tests.rs"))
        .expect("form numeric behavior tests should be readable");
    let paired = fs::read_to_string(manifest_dir.join("src/gui/form/paired.rs"))
        .expect("form paired helpers should be readable");
    let paired_tests = fs::read_to_string(manifest_dir.join("src/gui/form/paired/tests.rs"))
        .expect("form paired behavior tests should be readable");

    assert!(
        numeric.contains("pub struct DecimalTextInputPolicy")
            && numeric.contains("pub fn sanitize_decimal_text_insert")
            && numeric.contains("#[path = \"numeric/tests.rs\"]")
            && !numeric.contains("fn decimal_text_insert_keeps_digits_and_one_decimal_point"),
        "numeric form helpers should keep behavior tests delegated"
    );
    assert!(
        numeric_tests.contains("fn decimal_text_insert_keeps_digits_and_one_decimal_point")
            && numeric_tests.contains("fn rounded_scaled_u16_clamps_non_finite_and_large_values"),
        "numeric form behavior coverage should live in form/numeric/tests.rs"
    );
    assert!(
        paired.contains("pub enum PairedPickerTarget")
            && paired.contains("pub struct PairedStatusPanel")
            && paired.contains("#[path = \"paired/tests.rs\"]")
            && !paired.contains("fn paired_picker_models_cover_primary_and_secondary_fields"),
        "paired form helpers should keep behavior tests delegated"
    );
    assert!(
        paired_tests.contains("fn paired_picker_models_cover_primary_and_secondary_fields")
            && paired_tests.contains("fn paired_status_panel_returns_options_for_target"),
        "paired form behavior coverage should live in form/paired/tests.rs"
    );
}

#[test]
fn gui_core_state_primitives_keep_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let focus = fs::read_to_string(manifest_dir.join("src/gui/focus.rs"))
        .expect("gui focus primitive should be readable");
    let focus_tests = fs::read_to_string(manifest_dir.join("src/gui/focus/tests.rs"))
        .expect("gui focus behavior tests should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/gui/frame.rs"))
        .expect("gui frame feedback primitive should be readable");
    let frame_tests = fs::read_to_string(manifest_dir.join("src/gui/frame/tests.rs"))
        .expect("gui frame behavior tests should be readable");
    let selection = fs::read_to_string(manifest_dir.join("src/gui/selection.rs"))
        .expect("gui selection primitive should be readable");
    let selection_tests = fs::read_to_string(manifest_dir.join("src/gui/selection/tests.rs"))
        .expect("gui selection behavior tests should be readable");

    assert!(
        focus.contains("pub enum FocusSurface")
            && focus.contains("#[path = \"focus/tests.rs\"]")
            && !focus.contains("fn focus_surface_defaults_to_none"),
        "focus surface state should live in gui/focus.rs while behavior tests stay delegated"
    );
    assert!(
        focus_tests.contains("fn focus_surface_defaults_to_none"),
        "focus behavior coverage should live in gui/focus/tests.rs"
    );
    assert!(
        frame.contains("pub struct FrameBuildResult")
            && frame.contains("#[path = \"frame/tests.rs\"]")
            && !frame.contains("fn frame_build_result_defaults_to_no_work_observed"),
        "frame feedback state should live in gui/frame.rs while behavior tests stay delegated"
    );
    assert!(
        frame_tests.contains("fn frame_build_result_defaults_to_no_work_observed"),
        "frame behavior coverage should live in gui/frame/tests.rs"
    );
    assert!(
        selection.contains("pub enum TriState")
            && selection.contains("pub enum TriageTarget")
            && selection.contains("#[path = \"selection/tests.rs\"]")
            && !selection.contains("fn tri_state_defaults_to_off"),
        "selection state should live in gui/selection.rs while behavior tests stay delegated"
    );
    assert!(
        selection_tests.contains("fn tri_state_defaults_to_off")
            && selection_tests.contains("fn triage_target_names_generic_three_way_selection"),
        "selection behavior coverage should live in gui/selection/tests.rs"
    );
}

#[test]
fn text_line_layout_keeps_insets_and_placement_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/text_layout/mod.rs"))
        .expect("text layout module root should be readable");
    let insets = fs::read_to_string(manifest_dir.join("src/gui/text_layout/insets.rs"))
        .expect("text layout insets model should be readable");
    let placement = fs::read_to_string(manifest_dir.join("src/gui/text_layout/placement.rs"))
        .expect("text layout placement helpers should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/text_layout/tests.rs"))
        .expect("text layout behavior tests should be readable");

    assert!(
        root.contains("mod insets;")
            && root.contains("mod placement;")
            && root.contains("pub use insets::TextLineInsets;")
            && root.contains("pub use placement::snap_text_baseline_to_pixel;")
            && !root.contains("pub struct TextLineInsets")
            && !root.contains("fn inset_rect"),
        "text layout root should own public API wiring while insets and placement stay delegated"
    );
    assert!(
        insets.contains("pub struct TextLineInsets")
            && insets.contains("pub fn symmetric")
            && insets.contains("pub fn horizontal"),
        "text-line inset data should live in gui/text_layout/insets.rs"
    );
    assert!(
        placement.contains("pub(super) fn compute_text_line")
            && placement.contains("fn clamp_min_top")
            && placement.contains("fn inset_rect")
            && placement.contains("pub fn snap_text_baseline_to_pixel"),
        "text-line placement math should live in gui/text_layout/placement.rs"
    );
    assert!(
        tests.contains("fn centered_line_reuses_cached_geometry_for_identical_inputs")
            && tests.contains("fn snap_text_baseline_to_pixel_keeps_height_and_rounds_bottom_edge"),
        "text-line cache and placement behavior coverage should stay in gui/text_layout/tests.rs"
    );
}

#[test]
fn visualization_behavior_tests_stay_grouped_by_surface_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests.rs"))
        .expect("visualization test root should be readable");
    let spatial = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/spatial.rs"))
        .expect("spatial visualization tests should be readable");
    let canvas = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/canvas.rs"))
        .expect("canvas visualization tests should be readable");
    let signal = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/signal.rs"))
        .expect("signal visualization tests should be readable");
    let timeline = fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline.rs"))
        .expect("timeline visualization test root should be readable");
    let timeline_mapper =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/mapper.rs"))
            .expect("timeline mapper tests should be readable");
    let timeline_metadata =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/metadata.rs"))
            .expect("timeline metadata tests should be readable");
    let timeline_aggregate =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/aggregate.rs"))
            .expect("timeline aggregate tests should be readable");
    let timeline_fixtures =
        fs::read_to_string(manifest_dir.join("src/gui/visualization/tests/timeline/fixtures.rs"))
            .expect("timeline visualization test fixtures should be readable");

    assert!(
        root.contains("mod spatial;")
            && root.contains("mod canvas;")
            && root.contains("mod signal;")
            && root.contains("mod timeline;")
            && !root.contains("fn timeline_motion_state")
            && !root.contains("fn canvas_layer_hit_testing"),
        "visualization test root should index focused behavior groups instead of owning all visualization cases"
    );
    assert!(
        spatial.contains("fn normalized_milli_point_projects_and_clamps_into_rect")
            && canvas.contains("fn canvas_invalidation_splits_scene_and_interaction_rebuilds")
            && signal.contains("fn signal_tool_state_preserves_generic_interaction_flags")
            && timeline.contains("mod mapper;")
            && timeline.contains("mod metadata;")
            && timeline.contains("mod aggregate;")
            && timeline.contains("mod fixtures;")
            && !timeline.contains("fn timeline_motion_state_aggregates_surface_chrome_tools"),
        "visualization behavior tests should stay grouped by spatial, canvas, signal, and timeline concerns"
    );
    assert!(
        timeline_mapper.contains("fn timeline_coordinate_mapper_projects_and_back_projects_micros")
            && timeline_metadata.contains(
                "fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead"
            )
            && timeline_aggregate
                .contains("fn timeline_motion_state_aggregates_surface_chrome_tools")
            && timeline_fixtures.contains("fn timeline_viewport_parts"),
        "timeline visualization tests should stay grouped by mapper, metadata, aggregate, and fixture concerns"
    );
}

#[test]
fn signal_visualization_state_uses_named_parts_for_status_and_preview_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/signal.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let chrome = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/chrome.rs"))
        .expect("signal chrome state source should be readable");
    let preview = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/preview.rs"))
        .expect("signal raster preview source should be readable");
    let tools = fs::read_to_string(manifest_dir.join("src/gui/visualization/signal/tools.rs"))
        .expect("signal tool state source should be readable");

    for required in [
        "mod chrome;",
        "mod preview;",
        "mod tools;",
        "pub use chrome::{ChannelViewMode, SignalChromeParts, SignalChromeState};",
        "pub use preview::{SignalRasterPreview, SignalRasterPreviewParts};",
        "pub use tools::{SignalToolFlags, SignalToolState};",
    ] {
        assert!(
            source.contains(required),
            "signal visualization root should keep public re-exports while delegating `{required}`"
        );
    }

    assert!(
        chrome.contains("pub struct SignalChromeParts")
            && chrome.contains("pub fn from_parts(parts: SignalChromeParts) -> Self"),
        "signal chrome state should expose named parts for readable public construction"
    );
    assert!(
        preview.contains("pub struct SignalRasterPreviewParts")
            && preview.contains("pub fn from_parts(parts: SignalRasterPreviewParts) -> Self"),
        "signal raster preview state should expose named parts for readable public construction"
    );
    assert!(
        chrome.contains("Self::from_parts(SignalChromeParts {")
            && preview.contains("Self::from_parts(SignalRasterPreviewParts {"),
        "signal compatibility constructors should delegate through named parts objects"
    );
    assert!(
        !source.contains("pub struct SignalChromeState")
            && !source.contains("pub struct SignalRasterPreview")
            && !source.contains("pub struct SignalToolState")
            && chrome.contains("pub enum ChannelViewMode")
            && preview.contains("Arc<ImageRgba>")
            && tools.contains("pub struct SignalToolFlags")
            && tools.contains("pub fn from_flags(flags: SignalToolFlags) -> Self"),
        "signal chrome, raster preview, and tool flags should stay in focused visualization modules"
    );
}

#[test]
fn canvas_layer_state_uses_named_parts_for_hit_test_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/canvas.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct CanvasLayerParts")
            && source.contains("pub fn from_parts(parts: CanvasLayerParts) -> Self"),
        "canvas layer state should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(CanvasLayerParts {"),
        "the positional compatibility constructor should delegate through the named parts object"
    );
}

#[test]
fn split_pane_assigned_rows_use_named_parts_for_assignment_flags() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/panel/split_pane/assigned_row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct SplitPaneAssignment")
            && source.contains("pub struct SplitPaneAssignedRowParts")
            && source.contains("pub fn from_parts(parts: SplitPaneAssignedRowParts) -> Self"),
        "split-pane assigned rows should expose named parts for readable public construction"
    );
    assert!(
        source.contains("Self::from_parts(SplitPaneAssignedRowParts {")
            && source.contains("self.with_assignment(SplitPaneAssignment { upper, lower })"),
        "split-pane compatibility constructors should delegate through named assignment objects"
    );
}

#[test]
fn floating_panel_drags_use_named_parts_for_pointer_geometry() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/panel/floating.rs");
    let module_path = manifest_dir.join("src/gui/panel.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    assert!(
        source.contains("pub struct FloatingPanelDragParts")
            && source.contains("pub fn from_parts(parts: FloatingPanelDragParts) -> Self"),
        "floating-panel drags should expose named parts for panel rect and pointer geometry"
    );
    assert!(
        source.contains("Self::from_parts(FloatingPanelDragParts {")
            && module.contains("FloatingPanelDragParts"),
        "floating-panel drag compatibility constructor and panel export should keep the named-parts path available"
    );
}

#[test]
fn inline_badge_metrics_use_named_parts_for_geometry_tokens() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let badge = fs::read_to_string(manifest_dir.join("src/gui/badge.rs"))
        .expect("badge facade should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/badge/model.rs"))
        .expect("badge model module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/badge/tests.rs"))
        .expect("badge behavior tests should be readable");
    let root = fs::read_to_string(manifest_dir.join("src/gui/badge/inline.rs"))
        .expect("inline badge root should be readable");
    let metrics = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/metrics.rs"))
        .expect("inline badge metrics should be readable");
    let labels = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/labels.rs"))
        .expect("inline badge labels should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/badge/inline/geometry.rs"))
        .expect("inline badge geometry should be readable");

    assert!(
        badge.contains("mod model;")
            && badge.contains("pub use model::{PillEditorPanel, SelectablePill};")
            && badge.contains("#[path = \"badge/tests.rs\"]")
            && !badge.contains("pub struct SelectablePill")
            && !badge.contains("fn selectable_pill_preserves_identity_label_and_state"),
        "badge facade should re-export focused pill models and keep behavior tests out of the root module"
    );
    assert!(
        model.contains("pub struct SelectablePill")
            && model.contains("pub struct PillEditorPanel")
            && tests.contains("fn selectable_pill_preserves_identity_label_and_state")
            && tests.contains("fn inline_badge_rects_handle_empty_or_cramped_inputs"),
        "badge model DTOs and behavior tests should live in focused badge/model.rs and badge/tests.rs modules"
    );
    assert!(
        root.contains("mod geometry;")
            && root.contains("mod labels;")
            && root.contains("mod metrics;")
            && root.contains("pub use metrics::{InlineBadgeMetrics, InlineBadgeMetricsParts};"),
        "inline badge root should delegate metrics, label parsing, and geometry helpers"
    );
    assert!(
        metrics.contains("pub struct InlineBadgeMetricsParts")
            && metrics.contains("pub fn from_parts(parts: InlineBadgeMetricsParts) -> Self"),
        "inline badge metrics should expose named parts for readable public construction"
    );
    assert!(
        metrics.contains("Self::from_parts(InlineBadgeMetricsParts {"),
        "the positional compatibility constructor should delegate through the named metrics object"
    );
    assert!(
        labels.contains("pub fn inline_badge_labels")
            && labels.contains("pub fn inline_badge_labels_owned_into"),
        "inline badge label splitting and materialization should live in inline/labels.rs"
    );
    assert!(
        geometry.contains("pub fn inline_badge_rects_for_labels_into")
            && geometry.contains("pub fn inline_badge_text_origin")
            && geometry.contains("pub fn inline_badge_cluster_reserved_width"),
        "inline badge geometry and text placement should live in inline/geometry.rs"
    );
}

#[test]
fn timeline_metadata_state_uses_named_parts_for_projection_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");

    for (file, parts, from_parts, wrapper) in [
        (
            "transport.rs",
            "pub struct TimelineTransportParts",
            "pub fn from_parts(parts: TimelineTransportParts) -> Self",
            "Self::from_parts(TimelineTransportParts {",
        ),
        (
            "feedback.rs",
            "pub struct TimelineFeedbackParts",
            "pub fn from_parts(parts: TimelineFeedbackParts) -> Self",
            "Self::from_parts(TimelineFeedbackParts {",
        ),
        (
            "presentation.rs",
            "pub struct TimelinePresentationParts",
            "pub fn from_parts(parts: TimelinePresentationParts) -> Self",
            "Self::from_parts(TimelinePresentationParts {",
        ),
    ] {
        let source_path = timeline_dir.join(file);
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

        assert!(
            source.contains(parts) && source.contains(from_parts),
            "timeline metadata in {file} should expose named parts for readable public construction"
        );
        assert!(
            source.contains(wrapper),
            "timeline metadata compatibility constructor in {file} should delegate through named parts"
        );
    }
}

#[test]
fn timeline_viewport_uses_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/visualization/timeline/viewport.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct TimelineViewportParts")
            && source.contains("pub fn from_parts(parts: TimelineViewportParts) -> Self"),
        "timeline viewport should expose named parts for readable multi-precision bounds"
    );
    assert!(
        source.contains("Self::from_parts(TimelineViewportParts {")
            && source.contains("Self::from_parts(TimelineViewportParts::default())"),
        "timeline viewport compatibility/default constructors should delegate through named parts"
    );
}

#[test]
fn layout_constraints_use_named_parts_for_min_max_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/constraints.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/constraints/tests.rs"))
        .expect("layout constraint behavior tests should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    assert!(
        source.contains("pub struct ConstraintsParts")
            && source.contains("pub fn from_parts(parts: ConstraintsParts) -> Self"),
        "layout constraints should expose named parts for readable min/max bound construction"
    );
    assert!(
        source.contains("Self::from_parts(ConstraintsParts {")
            && module.contains("pub use constraints::{Constraints, ConstraintsParts};"),
        "layout constraint constructors and public exports should keep the named-parts path available"
    );
    assert!(
        source.contains("#[path = \"constraints/tests.rs\"]")
            && !source.contains("fn constraints_normalize_invalid_ranges"),
        "layout constraint behavior tests should stay delegated"
    );
    assert!(
        tests.contains("fn constraints_normalize_invalid_ranges"),
        "layout constraint behavior coverage should live in constraints/tests.rs"
    );
}

#[test]
fn layout_tree_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/tree.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SlotChildParts",
            "pub fn from_parts(parts: SlotChildParts) -> Self",
            "Self::from_parts(SlotChildParts {",
        ),
        (
            "pub struct ContainerNodeParts",
            "pub fn from_parts(parts: ContainerNodeParts) -> Self",
            "Self::from_parts(ContainerNodeParts {",
        ),
        (
            "pub struct WidgetNodeParts",
            "pub fn from_parts(parts: WidgetNodeParts) -> Self",
            "Self::from_parts(WidgetNodeParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "layout tree public nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        source.contains("pub fn container_from_parts(parts: ContainerNodeParts) -> Self")
            && source.contains("pub fn widget_from_parts(parts: WidgetNodeParts) -> Self")
            && module.contains("ContainerNodeParts")
            && module.contains("SlotChildParts")
            && module.contains("WidgetNodeParts"),
        "layout tree named parts should be available through the public layout module"
    );
}

#[test]
fn layout_tree_derived_state_keeps_metrics_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let derived = fs::read_to_string(manifest_dir.join("src/gui/layout_core/tree/derived.rs"))
        .expect("layout tree derived state module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/tree/derived/tests.rs"))
        .expect("layout tree derived state tests should be readable");

    assert!(
        derived.contains("fn container_derived_state")
            && derived.contains("KnownMainMetrics")
            && derived.contains("#[path = \"derived/tests.rs\"]")
            && !derived.contains("fn container_precomputes_uniform_main_size_with_extent"),
        "layout tree derived metrics should live in tree/derived.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn container_precomputes_uniform_main_size_with_extent")
            && tests.contains("fn container_does_not_mark_margin_rows_as_uniform"),
        "layout tree derived behavior coverage should live in tree/derived/tests.rs"
    );
}
