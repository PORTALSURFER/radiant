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
            && window.contains("struct ComputedVirtualWindow")
            && window.contains("fn compute_virtual_window")
            && window.contains("fn lower_bound_end")
            && window.contains("fn lower_bound_start")
            && !window.contains("-> (f32, f32, usize, usize, bool)"),
        "virtual-window binary search bounds should live in scroll_helpers/window.rs and return a named window model"
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
