use super::*;

#[test]
fn layout_virtual_cache_invalidation_stays_with_cache_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/mod.rs"))
        .expect("layout engine module should be readable");
    let cache = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/cache.rs"))
        .expect("layout cache module should be readable");

    assert!(
        engine.contains("invalidate_virtual_cache_for(&mut self.virtual_cache, node_id)")
            && engine.contains("invalidate_virtual_cache_for_any(&mut self.virtual_cache"),
        "layout engine should delegate virtual-cache invalidation to the cache module"
    );
    assert!(
        !engine.contains("fn invalidate_virtual_cache_for_any")
            && !engine.contains("entry.dependencies.iter()"),
        "layout engine orchestration should not own cached virtual-metric dependency scans"
    );
    assert!(
        cache.contains("fn invalidate_virtual_cache_for(")
            && cache.contains("fn invalidate_virtual_cache_for_any(")
            && cache.contains("impl CachedVirtualMetrics")
            && cache.contains("fn depends_on"),
        "virtual cache dependency invalidation should live with cached metric types"
    );
}

#[test]
fn layout_virtualization_tests_keep_fixture_and_behavior_groups_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/virtualization_tests.rs"))
            .expect("layout virtualization test root should be readable");
    let helpers = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/helpers.rs"),
    )
    .expect("layout virtualization test helpers should be readable");
    let performance = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/performance.rs"),
    )
    .expect("layout virtualization performance tests should be readable");
    let cache = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/cache.rs"),
    )
    .expect("layout virtualization cache tests should be readable");
    let diagnostics = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/diagnostics.rs"),
    )
    .expect("layout virtualization diagnostic tests should be readable");
    let alignment = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/alignment.rs"),
    )
    .expect("layout virtualization alignment tests should be readable");
    let equivalence = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/virtualization_tests/equivalence.rs"),
    )
    .expect("layout virtualization equivalence tests should be readable");
    let equivalence_mixed = fs::read_to_string(
        manifest_dir
            .join("src/gui/layout_core/engine/virtualization_tests/equivalence/mixed_sizing.rs"),
    )
    .expect("layout virtualization mixed-sizing equivalence tests should be readable");
    let equivalence_fixed = fs::read_to_string(
        manifest_dir
            .join("src/gui/layout_core/engine/virtualization_tests/equivalence/fixed_margins.rs"),
    )
    .expect("layout virtualization fixed-margin equivalence tests should be readable");

    for required in [
        "#[path = \"virtualization_tests/alignment.rs\"]",
        "#[path = \"virtualization_tests/cache.rs\"]",
        "#[path = \"virtualization_tests/diagnostics.rs\"]",
        "#[path = \"virtualization_tests/equivalence.rs\"]",
        "#[path = \"virtualization_tests/helpers.rs\"]",
        "#[path = \"virtualization_tests/performance.rs\"]",
    ] {
        assert!(
            root.contains(required),
            "layout virtualization test root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("fn scroll_virtualization_limits_materialized_nodes_for_large_lists")
            && !root.contains("fn virtualization_policy_is_ignored_for_unsupported_content_kind")
            && !root.contains("fn virtualization_supports_non_start_linear_alignment"),
        "layout virtualization test root should not own behavior bodies"
    );
    assert!(
        helpers.contains("fn scroll_with_content")
            && helpers.contains("fn fixed_virtualized_scroll_root")
            && performance
                .contains("fn scroll_virtualization_limits_materialized_nodes_for_large_lists")
            && performance
                .contains("fn fixed_size_virtualized_scroll_avoids_cold_full_list_measurement")
            && cache.contains("fn virtualized_metrics_cache_tracks_fixed_row_shape_changes")
            && diagnostics
                .contains("fn virtualization_policy_is_ignored_for_unsupported_content_kind")
            && diagnostics.contains("fn virtualization_debug_primitives_are_emitted")
            && diagnostics.contains("fn invalid_virtualization_overscan_is_clamped")
            && alignment.contains("fn virtualization_supports_non_start_linear_alignment")
            && equivalence.contains("mod mixed_sizing;")
            && equivalence.contains("mod fixed_margins;")
            && !equivalence.contains(
                "fn virtualized_fill_and_percent_layout_matches_full_layout_for_window_items"
            )
            && equivalence_mixed.contains(
                "fn virtualized_fill_and_percent_layout_matches_full_layout_for_window_items"
            )
            && equivalence_fixed
                .contains("fn virtualized_fixed_rows_with_balanced_margins_match_full_layout"),
        "layout virtualization coverage should stay grouped by behavior concern"
    );
}

#[test]
fn layout_scroll_virtual_window_search_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let helpers = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll_helpers.rs"),
    )
    .expect("layout scroll helpers should be readable");
    let window = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll_helpers/window.rs"),
    )
    .expect("layout scroll virtual window helper should be readable");
    let tests = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll_helpers/tests.rs"),
    )
    .expect("layout scroll helper tests should be readable");
    let virtualization = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/layout/scroll/virtualization.rs"),
    )
    .expect("layout scroll virtualization module should be readable");

    assert!(
        helpers.contains("mod window;")
            && helpers.contains("pub(super) use window::compute_virtual_window;")
            && virtualization.contains("compute_virtual_window"),
        "scroll virtualization should consume focused virtual-window search helpers through scroll_helpers"
    );
    assert!(
        !helpers.contains("fn lower_bound_end")
            && !helpers.contains("fn lower_bound_start")
            && window.contains("fn compute_virtual_window")
            && window.contains("fn lower_bound_end")
            && window.contains("fn lower_bound_start"),
        "virtual-window binary search bounds should live in scroll_helpers/window.rs"
    );
    assert!(
        helpers.contains("#[path = \"scroll_helpers/tests.rs\"]")
            && !helpers.contains("fn uniform_virtual_window_matches_visible_span_bounds"),
        "layout scroll helper behavior tests should stay delegated"
    );
    assert!(
        tests.contains("fn uniform_virtual_window_matches_visible_span_bounds"),
        "layout scroll helper behavior coverage should live in scroll_helpers/tests.rs"
    );
}

#[test]
fn surface_paint_plan_buffering_stays_with_capacity_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let projection = fs::read_to_string(manifest_dir.join("src/runtime/surface/projection.rs"))
        .expect("surface paint projection should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/runtime/controller/context/frame.rs"))
        .expect("runtime frame paint projection should be readable");
    let capacity = fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity.rs"))
        .expect("surface paint capacity policy should be readable");
    let capacity_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity/tests.rs"))
            .expect("surface paint capacity tests should be readable");

    assert!(
        capacity.contains("fn empty_paint_plan_for_layout")
            && capacity.contains("fn clear_paint_plan_for_layout")
            && capacity.contains("fn estimated_paint_primitive_capacity")
            && capacity.contains("#[path = \"capacity/tests.rs\"]")
            && !capacity.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts"),
        "layout-aware paint-plan buffer lifecycle should live with the capacity policy while behavior tests stay delegated"
    );
    assert!(
        capacity_tests.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts")
            && capacity_tests.contains("fn clear_paint_plan_for_layout_reuses_existing_capacity"),
        "surface paint capacity behavior coverage should live in surface/paint/capacity/tests.rs"
    );
    assert!(
        projection.contains("empty_paint_plan_for_layout(layout, theme)")
            && projection.contains("clear_paint_plan_for_layout(plan, layout, theme)")
            && frame.contains("empty_paint_plan_for_layout(&self.layout, theme)"),
        "surface and runtime paint projection should consume layout-aware plan helpers"
    );
    assert!(
        !projection.contains("estimated_paint_primitive_capacity")
            && !frame.contains("estimated_paint_primitive_capacity")
            && !projection.contains("SurfacePaintPlan::empty_with_capacity")
            && !frame.contains("SurfacePaintPlan::empty_with_capacity"),
        "paint projection callers should not duplicate capacity-policy mechanics"
    );
}

#[test]
fn runtime_paint_primitive_support_keeps_models_queries_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stats = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/stats.rs"))
        .expect("paint primitive stats module should be readable");
    let stats_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/stats/tests.rs"))
            .expect("paint primitive stats tests should be readable");
    let query = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/query.rs"))
        .expect("paint primitive query module should be readable");
    let query_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/query/tests.rs"))
            .expect("paint primitive query tests should be readable");
    let path = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/path.rs"))
        .expect("paint primitive path module should be readable");
    let path_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/path/tests.rs"))
            .expect("paint primitive path tests should be readable");
    let plan = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/plan.rs"))
        .expect("paint primitive plan module should be readable");
    let plan_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/plan/tests.rs"))
            .expect("paint primitive plan tests should be readable");
    let text = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/text.rs"))
        .expect("paint primitive text module should be readable");
    let text_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/text/tests.rs"))
            .expect("paint primitive text tests should be readable");

    assert!(
        stats.contains("pub struct SurfacePaintStats")
            && stats.contains("pub fn stats(&self) -> SurfacePaintStats")
            && stats.contains("#[path = \"stats/tests.rs\"]")
            && !stats.contains("fn surface_paint_plan_stats_count_core_primitive_groups"),
        "paint primitive stats should live in primitives/stats.rs while behavior tests stay delegated"
    );
    assert!(
        stats_tests.contains("fn surface_paint_plan_stats_count_core_primitive_groups"),
        "paint primitive stats behavior coverage should live in primitives/stats/tests.rs"
    );
    assert!(
        query.contains("pub fn first_widget_rect(&self, widget_id: WidgetId) -> Option<Rect>")
            && query.contains("pub fn widget_id(&self) -> Option<WidgetId>")
            && query.contains("pub fn rect(&self) -> Option<Rect>")
            && query.contains("#[path = \"query/tests.rs\"]")
            && !query
                .contains("fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order"),
        "paint primitive query helpers should live in primitives/query.rs while behavior tests stay delegated"
    );
    assert!(
        query_tests.contains("fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order")
            && query_tests
                .contains("fn paint_primitive_reports_widget_id_and_rect_for_anchor_primitives"),
        "paint primitive query behavior coverage should live in primitives/query/tests.rs"
    );
    assert!(
        path.contains("pub struct PaintPath")
            && path.contains("pub struct PaintTransform")
            && path.contains("pub enum PaintFillRule")
            && path.contains("#[path = \"path/tests.rs\"]")
            && !path.contains("fn paint_path_preserves_backend_neutral_commands"),
        "paint path models should live in primitives/path.rs while behavior tests stay delegated"
    );
    assert!(
        path_tests.contains("fn paint_path_preserves_backend_neutral_commands")
            && path_tests.contains("fn paint_transform_reports_finite_coefficients"),
        "paint path behavior coverage should live in primitives/path/tests.rs"
    );
    assert!(
        plan.contains("pub enum PaintPrimitive")
            && plan.contains("pub struct SurfacePaintPlan")
            && plan.contains("pub struct TransientOverlayContext")
            && plan.contains("#[path = \"plan/tests.rs\"]")
            && !plan.contains("fn empty_with_capacity_presizes_primitive_storage"),
        "paint plan models should live in primitives/plan.rs while behavior tests stay delegated"
    );
    assert!(
        plan_tests.contains("fn empty_with_capacity_presizes_primitive_storage")
            && plan_tests.contains("fn clear_for_theme_with_capacity_grows_to_requested_capacity"),
        "paint plan behavior coverage should live in primitives/plan/tests.rs"
    );
    assert!(
        text.contains("pub struct PaintText")
            && text.contains("pub struct PaintTextRun")
            && text.contains("pub struct PaintTextInput")
            && text.contains("#[path = \"text/tests.rs\"]")
            && !text.contains("fn paint_text_converts_compares_and_shares_storage"),
        "paint text models should live in primitives/text.rs while behavior tests stay delegated"
    );
    assert!(
        text_tests.contains("fn paint_text_converts_compares_and_shares_storage"),
        "paint text behavior coverage should live in primitives/text/tests.rs"
    );
}

#[test]
fn runtime_scroll_support_keeps_affordance_and_hit_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paint_scroll = fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll.rs"))
        .expect("runtime paint scroll helpers should be readable");
    let paint_scroll_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll/tests.rs"))
            .expect("runtime paint scroll tests should be readable");
    let controller_scrollbar =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar.rs"))
            .expect("runtime controller scrollbar helpers should be readable");
    let controller_scrollbar_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar/tests.rs"))
            .expect("runtime controller scrollbar tests should be readable");

    assert!(
        paint_scroll.contains("struct ScrollAffordance")
            && paint_scroll.contains("fn push_scroll_affordance")
            && paint_scroll.contains("fn resolve_scroll_affordance")
            && paint_scroll.contains("#[path = \"scroll/tests.rs\"]")
            && !paint_scroll.contains("fn scroll_affordance_clamps_thumb_to_cramped_track"),
        "runtime scroll paint affordance helpers should live in paint/scroll.rs while behavior tests stay delegated"
    );
    assert!(
        paint_scroll_tests.contains("fn scroll_affordance_clamps_thumb_to_cramped_track")
            && paint_scroll_tests.contains("fn scroll_affordance_rejects_nonfinite_layout_rects"),
        "runtime scroll paint behavior coverage should live in paint/scroll/tests.rs"
    );
    assert!(
        controller_scrollbar.contains("fn scrollbar_hit_column_contains_point")
            && controller_scrollbar.contains("fn scrollbar_thumb_hit_rect")
            && controller_scrollbar.contains("#[path = \"scrollbar/tests.rs\"]")
            && !controller_scrollbar
                .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar hit helpers should live in controller/scroll/scrollbar.rs while behavior tests stay delegated"
    );
    assert!(
        controller_scrollbar_tests
            .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar behavior coverage should live in controller/scroll/scrollbar/tests.rs"
    );
}

#[test]
fn surface_layout_projection_records_traversal_through_index_methods() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout = fs::read_to_string(manifest_dir.join("src/runtime/surface/layout.rs"))
        .expect("surface layout projection should be readable");
    let index = fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index.rs"))
        .expect("surface traversal index should be readable");
    let records =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/records.rs"))
            .expect("surface traversal records should be readable");
    let recording =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/recording.rs"))
            .expect("surface traversal recording helpers should be readable");
    let capacity =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/capacity.rs"))
            .expect("surface traversal capacity helpers should be readable");
    let index_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/traversal/index/tests.rs"))
            .expect("surface traversal index tests should be readable");
    let capacity_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/surface/traversal/index/capacity/tests.rs"),
    )
    .expect("surface traversal capacity tests should be readable");

    assert!(
        index.contains("mod capacity;")
            && index.contains("mod recording;")
            && index.contains("mod records;")
            && index.contains("#[path = \"index/tests.rs\"]")
            && index.contains("pub(in crate::runtime) use records::{")
            && !index.contains("fn record_container")
            && !index.contains("fn record_widget")
            && !index.contains("fn traversal_records_route_to_expected_buckets"),
        "surface traversal index root should delegate traversal bucket mutation helpers and behavior tests"
    );
    assert!(
        index_tests.contains("fn traversal_records_route_to_expected_buckets"),
        "surface traversal index behavior coverage should live in traversal/index/tests.rs"
    );
    assert!(
        records.contains("struct SurfaceContainerTraversalRecord")
            && records.contains("struct SurfaceWidgetTraversalRecord")
            && !index.contains("struct SurfaceContainerTraversalRecord")
            && !index.contains("struct SurfaceWidgetTraversalRecord"),
        "surface traversal record DTOs should live in traversal/index/records.rs"
    );
    assert!(
        capacity.contains("fn widget_clip_capacity")
            && capacity.contains("fn reserve_vec_capacity")
            && capacity.contains("fn reserve_map_capacity")
            && capacity.contains("fn reserve_set_capacity")
            && capacity.contains("#[path = \"capacity/tests.rs\"]")
            && !index.contains("fn reserve_vec_capacity")
            && !index.contains("fn reserve_map_capacity")
            && !index.contains("fn reserve_set_capacity")
            && !capacity.contains("fn widget_clip_capacity_is_zero_without_scroll_containers"),
        "surface traversal capacity and reuse helpers should live in traversal/index/capacity.rs while behavior tests stay delegated"
    );
    assert!(
        capacity_tests.contains("fn widget_clip_capacity_is_zero_without_scroll_containers")
            && capacity_tests
                .contains("fn widget_clip_capacity_tracks_widgets_when_scroll_containers_exist"),
        "surface traversal capacity behavior coverage should live in traversal/index/capacity/tests.rs"
    );
    assert!(
        recording.contains("fn record_container")
            && recording.contains("fn record_widget")
            && recording.contains(".widget_paint_order.push")
            && recording.contains(".scroll_content_by_container.insert")
            && recording.contains(".container_hover_suppression.insert"),
        "surface traversal bucket recording should live in traversal/index/recording.rs"
    );
    assert!(
        layout.contains("traversal.record_container(SurfaceContainerTraversalRecord")
            && layout.contains("traversal.record_widget(SurfaceWidgetTraversalRecord"),
        "surface layout projection should describe traversal records instead of mutating buckets directly"
    );
    for forbidden in [
        ".widget_paint_order.push",
        ".widget_paths",
        ".focusable_widget_order.push",
        ".keyboard_focus_order.push",
        ".pointer_hit_order.push",
        ".wheel_hit_order.push",
        ".stateful_widget_order.push",
        ".container_hover_suppression",
        ".widget_clip_ancestors",
        ".container_clip_ancestors",
        ".scroll_container_order.push",
        ".scroll_content_by_container",
        ".styled_container_order.push",
    ] {
        assert!(
            !layout.contains(forbidden),
            "surface layout projection should not directly mutate traversal bucket `{forbidden}`"
        );
    }
}

#[test]
fn runtime_layout_refresh_delegates_traversal_state_handoff() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let layout = fs::read_to_string(manifest_dir.join("src/runtime/controller/state/layout.rs"))
        .expect("runtime layout state should be readable");
    let traversal =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/state/traversal.rs"))
            .expect("runtime traversal state should be readable");

    assert!(
        layout.contains("self.install_traversal_index(traversal)")
            && layout.contains("self.refresh_visible_traversal_orders()"),
        "runtime layout refresh should delegate traversal bucket installation and visible-order refresh"
    );
    assert!(
        traversal.contains("fn install_traversal_index")
            && traversal.contains("fn take_reusable_traversal_index")
            && traversal.contains("fn refresh_visible_traversal_orders"),
        "runtime traversal state handoff should live in a focused helper module"
    );
    for forbidden in [
        "self.widget_hit_order = traversal.",
        "self.widget_paths = traversal.",
        "set_order(traversal.",
        "self.container_hover_suppression = traversal.",
        "self.stateful_widget_order = traversal.",
        "self.widget_clip_ancestors = traversal.",
        "self.container_clip_ancestors = traversal.",
        "self.scroll_content_by_container = traversal.",
    ] {
        assert!(
            !layout.contains(forbidden),
            "runtime layout refresh should not directly install traversal bucket `{forbidden}`"
        );
    }
}
