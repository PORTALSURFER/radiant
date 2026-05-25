use super::*;

#[test]
fn layout_row_helpers_keep_geometry_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers.rs"))
        .expect("layout row helpers should be readable");
    let rects = fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers/rects.rs"))
        .expect("layout row helper rects should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers/tests.rs"))
        .expect("layout row helper tests should be readable");
    let fixed_rects = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/row_helpers/tests/fixed_rects.rs"),
    )
    .expect("layout fixed row rect tests should be readable");
    let fitting =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers/tests/fitting.rs"))
            .expect("layout row fitting tests should be readable");
    let stacked_rows = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/row_helpers/tests/stacked_rows.rs"),
    )
    .expect("layout stacked row tests should be readable");
    let widths =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers/tests/widths.rs"))
            .expect("layout row width tests should be readable");

    assert!(
        root.contains("mod fitting;")
            && root.contains("mod rects;")
            && root.contains("mod widths;")
            && root.contains("#[path = \"row_helpers/tests.rs\"]")
            && root.contains("#[path = \"row_helpers/tests.rs\"]")
            && !root.contains("fn fixed_width_row_rects_start_places_items_from_left_edge"),
        "layout row helper root should re-export focused geometry modules while behavior tests stay delegated"
    );
    assert!(
        tests.contains("#[path = \"tests/fixed_rects.rs\"]")
            && tests.contains("#[path = \"tests/fitting.rs\"]")
            && tests.contains("#[path = \"tests/stacked_rows.rs\"]")
            && tests.contains("#[path = \"tests/widths.rs\"]")
            && !tests.contains("fn fixed_width_row_rects_start_places_items_from_left_edge")
            && fixed_rects.contains("fn fixed_width_row_rects_start_places_items_from_left_edge")
            && fitting.contains("fn visible_suffix_widths_normalizes_negative_dimensions")
            && fitting.contains(
                "fn fixed_width_item_extent_for_available_width_fits_items_after_reserved_gaps"
            )
            && widths.contains("fn grouped_fixed_width_row_width_counts_visible_groups_and_gaps"),
        "layout row helper behavior coverage should live in focused row_helpers/tests modules"
    );
    assert!(
        rects.contains("pub struct StackedRowRectsParts")
            && rects.contains("pub fn stacked_row_rects_from_parts(")
            && rects.contains("pub fn stacked_row_rects_into_from_parts(")
            && rects.contains("stacked_row_rects_from_parts(StackedRowRectsParts {")
            && root.contains("StackedRowRectsParts"),
        "stacked-row geometry should expose named parts while preserving compatibility helpers"
    );
    assert!(
        stacked_rows.contains("fn stacked_row_rects_compatibility_helper_delegates_to_named_parts")
            && stacked_rows.contains(
                "fn stacked_row_rects_into_compatibility_helper_delegates_to_named_parts"
            ),
        "stacked-row tests should cover named-parts construction and compatibility wrappers"
    );
}

#[test]
fn layout_axis_helpers_keep_orientation_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let axis = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/helpers/axis.rs"))
        .expect("layout axis helper should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/helpers/axis/tests.rs"))
            .expect("layout axis helper tests should be readable");

    assert!(
        axis.contains("enum LayoutAxis")
            && axis.contains("fn main_extent")
            && axis.contains("fn overflow_flags")
            && axis.contains("#[path = \"axis/tests.rs\"]")
            && !axis.contains("fn layout_axis_resolves_main_and_cross_extents"),
        "layout axis orientation helpers should live in axis.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn layout_axis_resolves_main_and_cross_extents")
            && tests.contains("fn layout_axis_reports_overflow_direction"),
        "layout axis helper behavior coverage should live in axis/tests.rs"
    );
}

#[test]
fn layout_engine_root_tests_stay_grouped_by_behavior_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests.rs"))
        .expect("layout engine test root should be readable");
    let layout =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout.rs"))
            .expect("layout engine layout tests should be readable");
    let layout_row =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/row.rs"))
            .expect("layout engine row layout tests should be readable");
    let layout_switch =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/switch.rs"))
            .expect("layout engine switch layout tests should be readable");
    let layout_flow =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/flow.rs"))
            .expect("layout engine flow layout tests should be readable");
    let scroll =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scroll.rs"))
            .expect("layout engine scroll tests should be readable");
    let diagnostics =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/diagnostics.rs"))
            .expect("layout engine diagnostic tests should be readable");
    let debug = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/debug.rs"))
        .expect("layout engine debug tests should be readable");
    let scratch =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch.rs"))
            .expect("layout engine scratch test root should be readable");
    let scratch_reuse =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch/reuse.rs"))
            .expect("layout engine scratch reuse tests should be readable");
    let scratch_pruning = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/tests/scratch/pruning.rs"),
    )
    .expect("layout engine scratch pruning tests should be readable");
    let scratch_dirty =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch/dirty.rs"))
            .expect("layout engine scratch dirty tests should be readable");
    let scratch_fixtures = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/tests/scratch/fixtures.rs"),
    )
    .expect("layout engine scratch fixtures should be readable");

    assert!(
        root.contains("mod layout;")
            && root.contains("mod scroll;")
            && root.contains("mod diagnostics;")
            && root.contains("mod debug;")
            && root.contains("#[path = \"tests/scratch.rs\"]")
            && !root.contains("fn layout_tree_is_deterministic")
            && !root.contains("fn scroll_offset_is_clamped"),
        "layout engine test root should index focused behavior groups instead of owning all core layout cases"
    );
    assert!(
        scratch.contains("mod reuse;")
            && scratch.contains("mod pruning;")
            && scratch.contains("mod dirty;")
            && scratch.contains("mod fixtures;")
            && !scratch.contains("fn layout_engine_reuses_scratch_maps_between_passes"),
        "layout engine scratch tests should index focused scratch behavior groups instead of owning all cases"
    );
    assert!(
        layout.contains("mod row;")
            && layout.contains("mod switch;")
            && layout.contains("mod flow;")
            && !layout.contains("fn fill_children_redistribute_after_constrained_child_clamps")
            && scroll.contains("fn scroll_offset_is_clamped_and_reported")
            && diagnostics.contains("fn contradictory_constraints_emit_diagnostic")
            && debug.contains("fn debug_primitives_are_emitted_when_enabled"),
        "layout engine tests should stay grouped by layout, scroll, diagnostics, and debug concerns"
    );
    assert!(
        layout_row.contains("fn layout_tree_is_deterministic_for_same_input")
            && layout_row.contains("fn fill_children_redistribute_after_constrained_child_clamps")
            && layout_switch.contains("fn switch_layout_selects_breakpoint_child")
            && layout_flow.contains("fn wrap_layout_moves_items_to_next_line")
            && layout_flow.contains("fn grid_layout_places_items_by_row_and_column"),
        "layout engine layout tests should stay grouped by row/fill, switch, and flow container behavior"
    );
    assert!(
        scratch_reuse.contains("fn layout_engine_reuses_scratch_maps_between_passes")
            && scratch_pruning.contains("fn layout_engine_prunes_stale_measure_cache_versions")
            && scratch_dirty.contains(
                "fn dirty_subtree_invalidates_virtual_metrics_cache_for_whole_marked_set"
            )
            && scratch_fixtures.contains("fn fixed_virtualized_root"),
        "layout engine scratch tests should stay grouped by reuse, pruning, dirty-subtree, and fixture concerns"
    );
}
