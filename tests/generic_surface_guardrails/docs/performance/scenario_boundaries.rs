use super::support::read_project_file;

#[test]
fn layout_perf_scenarios_keep_synthetic_trees_separate() {
    let layout_scenarios = read_project_file("benches/perf_harness/layout_scenarios.rs");
    let layout_trees = read_project_file("benches/perf_harness/layout_scenarios/trees.rs");

    assert!(
        layout_scenarios.contains("layout_scenarios/trees.rs")
            && layout_trees.contains("deep_nesting_tree")
            && layout_trees.contains("wrap_tree")
            && layout_trees.contains("virtualized_scroll_tree"),
        "layout perf scenarios should keep scenario state separate from synthetic layout trees"
    );
}

#[test]
fn runtime_surface_perf_scenarios_keep_fixtures_and_commands_focused() {
    let runtime_surface = read_project_file("benches/perf_harness/runtime_scenarios/surface.rs");
    let runtime_scenarios_root = read_project_file("benches/perf_harness/runtime_scenarios.rs");
    let runtime_surface_nodes =
        read_project_file("benches/perf_harness/runtime_scenarios/surface/nodes.rs");
    let runtime_command_flattening =
        read_project_file("benches/perf_harness/runtime_scenarios/surface/command_flattening.rs");
    let runtime_invalidation =
        read_project_file("benches/perf_harness/runtime_scenarios/invalidation.rs");

    assert!(
        runtime_surface.contains("surface/nodes.rs")
            && runtime_surface.contains("surface/command_flattening.rs")
            && runtime_surface_nodes.contains("runtime_surface_node")
            && runtime_surface_nodes.contains("text_paint_surface_node")
            && runtime_surface_nodes.contains("horizontal_scroll_surface_node")
            && runtime_command_flattening.contains("Command::batch"),
        "runtime surface perf scenarios should keep scenario state, synthetic trees, and command flattening in focused modules"
    );
    assert!(
        runtime_scenarios_root.contains("runtime_scenarios/invalidation.rs")
            && runtime_invalidation.contains("RetainedSegmentPlan")
            && runtime_invalidation.contains("RetainedSegmentRevisions")
            && runtime_invalidation.contains("requires_static_rebuild")
            && runtime_invalidation.contains("requires_overlay_rebuild")
            && runtime_invalidation.contains("bump_revisions")
            && runtime_invalidation.contains("retained_segment_invalidation_1k"),
        "runtime invalidation perf scenarios should exercise retained segment masks and revision bumps"
    );
}

#[test]
fn runtime_virtualized_perf_scenarios_keep_bridge_fixtures_focused() {
    let runtime_virtualized =
        read_project_file("benches/perf_harness/runtime_scenarios/virtualized.rs");
    let runtime_virtualized_bridges =
        read_project_file("benches/perf_harness/runtime_scenarios/virtualized/bridges.rs");

    assert!(
        runtime_virtualized.contains("virtualized/bridges.rs")
            && runtime_virtualized_bridges.contains("VirtualWheelBridge")
            && runtime_virtualized_bridges.contains("NestedScrollBridge")
            && runtime_virtualized_bridges.contains("virtual_button_rows")
            && runtime_virtualized_bridges.contains("nested_scroll_rows"),
        "runtime virtualized perf scenarios should keep scenario state separate from synthetic bridge trees"
    );
}

#[test]
fn resource_and_text_perf_scenarios_cover_reusable_domain_logic() {
    let bench = read_project_file("benches/perf_harness.rs");
    let resource_scenarios = read_project_file("benches/perf_harness/resource_scenarios.rs");
    let text_scenarios = read_project_file("benches/perf_harness/text_scenarios.rs");

    assert!(
        bench.contains("perf_harness/resource_scenarios.rs")
            && resource_scenarios.contains("ResourceSlot::new")
            && resource_scenarios.contains("apply_for")
            && resource_scenarios.contains("resource_slot_stale_completions_1k"),
        "resource perf scenarios should exercise stale background completion rejection"
    );
    assert!(
        text_scenarios.contains("TextLineLayoutCache")
            && text_scenarios.contains("centered_text_line_with_cache")
            && text_scenarios.contains("top_text_line_with_cache")
            && text_scenarios.contains("TextInputState")
            && text_scenarios.contains("select_word_at")
            && text_scenarios.contains("selected_text_slice")
            && text_scenarios.contains("TextEditCommand::DeleteWordLeft")
            && text_scenarios.contains("TextEditCommand::DeleteWordRight"),
        "text perf scenarios should exercise the reusable text-line layout cache, text-input word selection, and word deletion"
    );
}
