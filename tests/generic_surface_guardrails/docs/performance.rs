use super::*;

#[test]
fn performance_harness_is_registered_and_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let bench = fs::read_to_string(manifest_dir.join("benches/perf_harness.rs"))
        .expect("perf_harness bench should be readable");
    let catalog = fs::read_to_string(manifest_dir.join("benches/perf_harness/catalog.rs"))
        .expect("perf_harness catalog should be readable");
    let runner = fs::read_to_string(manifest_dir.join("benches/perf_harness/runner.rs"))
        .expect("perf_harness runner should be readable");
    let layout_scenarios =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/layout_scenarios.rs"))
            .expect("layout bench scenarios should be readable");
    let layout_trees =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/layout_scenarios/trees.rs"))
            .expect("layout bench tree fixtures should be readable");
    let runtime_surface =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/runtime_scenarios/surface.rs"))
            .expect("runtime surface bench scenarios should be readable");
    let runtime_scenarios_root =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/runtime_scenarios.rs"))
            .expect("runtime scenarios root should be readable");
    let runtime_surface_nodes = fs::read_to_string(
        manifest_dir.join("benches/perf_harness/runtime_scenarios/surface/nodes.rs"),
    )
    .expect("runtime surface bench node fixtures should be readable");
    let runtime_command_flattening = fs::read_to_string(
        manifest_dir.join("benches/perf_harness/runtime_scenarios/surface/command_flattening.rs"),
    )
    .expect("runtime command flattening bench should be readable");
    let runtime_invalidation = fs::read_to_string(
        manifest_dir.join("benches/perf_harness/runtime_scenarios/invalidation.rs"),
    )
    .expect("runtime invalidation bench scenarios should be readable");
    let runtime_virtualized = fs::read_to_string(
        manifest_dir.join("benches/perf_harness/runtime_scenarios/virtualized.rs"),
    )
    .expect("runtime virtualized bench scenarios should be readable");
    let runtime_virtualized_bridges = fs::read_to_string(
        manifest_dir.join("benches/perf_harness/runtime_scenarios/virtualized/bridges.rs"),
    )
    .expect("runtime virtualized bench bridges should be readable");
    let resource_scenarios =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/resource_scenarios.rs"))
            .expect("resource bench scenarios should be readable");
    let text_scenarios =
        fs::read_to_string(manifest_dir.join("benches/perf_harness/text_scenarios.rs"))
            .expect("text bench scenarios should be readable");
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");

    for required in [
        "[[bench]]",
        "name = \"perf_harness\"",
        "path = \"benches/perf_harness.rs\"",
        "harness = false",
    ] {
        assert!(
            manifest.contains(required),
            "Cargo.toml should register perf harness with `{required}`"
        );
    }
    assert!(
        bench.contains("perf_harness/catalog.rs")
            && bench.contains("perf_harness/runner.rs")
            && bench.contains("catalog::run_registered_scenarios")
            && bench.contains("runner::print_scenario_list"),
        "perf_harness entrypoint should delegate scenario ownership to the catalog and runner"
    );
    let perf_scenarios = [
        "layout_deep_nesting",
        "layout_wrap_1k",
        "layout_virtualized_10k",
        "layout_virtualized_fixed_10k",
        "layout_virtualized_fixed_scroll_10k",
        "layout_mark_dirty_subtree_10k",
        "app_virtual_list_projection_10k",
        "app_virtual_list_projection_generated_child_ids_10k",
        "app_virtual_selectable_list_projection_10k",
        "app_virtual_list_window_projection_10k",
        "runtime_surface_large_tree",
        "runtime_text_paint_plan_1k",
        "runtime_horizontal_scroll_paint_1k",
        "runtime_virtualized_list_wheel_10k",
        "runtime_virtualized_list_hover_10k",
        "runtime_virtualized_list_stable_hover_10k",
        "runtime_virtualized_list_hover_paint_10k",
        "runtime_pointer_overlay_paint_10k",
        "runtime_retained_segment_invalidation_1k",
        "runtime_virtualized_nested_scroll_hover_10k",
        "runtime_refresh_large_tree",
        "runtime_resize_large_tree",
        "runtime_command_flattening_512",
        "runtime_command_drain_1k",
        "runtime_nested_command_drain_1k",
        "resource_slot_stale_completions_1k",
        "text_line_cache_1k",
        "text_word_selection_1k",
        "text_word_deletion_1k",
        "gpu_signal_summary",
        "gpu_surface_projection",
        "gpu_custom_shader_projection",
    ];
    for scenario in perf_scenarios {
        assert!(
            catalog.contains(scenario),
            "perf_harness catalog should include `{scenario}`"
        );
        let scenario_literal = format!("\"{scenario}\"");
        assert_eq!(
            catalog.matches(&scenario_literal).count(),
            1,
            "perf_harness catalog should register `{scenario}` once"
        );
        assert!(
            docs.contains(scenario),
            "docs/API.md should document perf scenario `{scenario}`"
        );
    }
    assert!(
        runner.contains("radiant_perf scenario="),
        "perf_harness runner should print parseable metric lines"
    );
    assert!(
        runner.contains("--jsonl")
            && runner.contains("--baseline-jsonl")
            && runner.contains("--write-baseline-jsonl")
            && runner.contains("--fail-on-baseline-regression")
            && runner.contains("OutputFormat::JsonLines")
            && runner.contains("BaselineSet")
            && runner.contains("BaselineOutput")
            && runner.contains("baseline_output_from_args")
            && runner.contains("fail_on_baseline_regression")
            && runner.contains("has_regression")
            && runner.contains("baseline_metric_json_line")
            && runner.contains("std::process::exit(1)")
            && runner.contains("\\\"type\\\":\\\"radiant_perf\\\"")
            && runner.contains("\\\"total_us\\\"")
            && runner.contains("\\\"avg_us\\\"")
            && runner.contains("\\\"baseline_avg_us\\\"")
            && runner.contains("\\\"baseline_ratio\\\"")
            && runner.contains("\\\"baseline_status\\\"")
            && runner.contains("BaselineSummary")
            && runner.contains("radiant_perf_summary")
            && runner.contains("baseline_missing")
            && runner.contains("\\\"baseline_slower\\\"")
            && runner.contains("MetricComparison::Missing")
            && runner.contains("baseline_status=missing")
            && runner.contains("\\\"baseline_status\\\":\\\"missing\\\""),
        "perf_harness runner should expose JSON-lines metrics, baseline comparison fields, missing-baseline status, and summary counts for trend capture"
    );
    assert!(
        runner.contains("--list") && runner.contains("radiant_perf scenarios:"),
        "perf_harness runner should expose a cheap scenario-listing mode"
    );
    assert!(
        catalog.contains("ScenarioSpec::new")
            && catalog.contains("\"layout\"")
            && catalog.contains("\"application_projection\"")
            && catalog.contains("\"runtime_surface\"")
            && catalog.contains("\"runtime_virtualized\"")
            && catalog.contains("\"runtime_invalidation\"")
            && catalog.contains("\"runtime_commands\"")
            && catalog.contains("\"resource_lifecycle\"")
            && catalog.contains("\"text\"")
            && catalog.contains("\"gpu_data\"")
            && catalog.contains("\"gpu_surface\"")
            && runner.contains("struct ScenarioSpec")
            && runner.contains("category={}")
            && runner.contains("iterations={}"),
        "perf_harness scenario listing should expose target-area categories and iteration counts"
    );
    assert!(
        layout_scenarios.contains("layout_scenarios/trees.rs")
            && layout_trees.contains("deep_nesting_tree")
            && layout_trees.contains("wrap_tree")
            && layout_trees.contains("virtualized_scroll_tree"),
        "layout perf scenarios should keep scenario state separate from synthetic layout trees"
    );
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
    assert!(
        runtime_virtualized.contains("virtualized/bridges.rs")
            && runtime_virtualized_bridges.contains("VirtualWheelBridge")
            && runtime_virtualized_bridges.contains("NestedScrollBridge")
            && runtime_virtualized_bridges.contains("virtual_button_rows")
            && runtime_virtualized_bridges.contains("nested_scroll_rows"),
        "runtime virtualized perf scenarios should keep scenario state separate from synthetic bridge trees"
    );
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
    assert!(
        bench.contains("bench_gpu_custom_shader_projection")
            && bench.contains("GpuShaderSurfaceDescriptor::new")
            && bench.contains("GpuSurfaceContent::CustomShader"),
        "perf_harness should exercise backend-neutral custom shader GPU surface projection"
    );
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_docs.contains("cargo bench --bench perf_harness")
            && normalized_docs.contains("cargo bench --bench perf_harness -- --list")
            && normalized_docs.contains(
                "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl"
            )
            && normalized_docs.contains(
                "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl"
            )
            && normalized_docs.contains(
                "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl"
            )
            && normalized_docs.contains(
                "Each JSON line includes `type`, `scenario`, `iterations`, `total_us`, and `avg_us`"
            )
            && normalized_docs.contains("Capture a machine-local baseline artifact directly")
            && normalized_docs
                .contains("`baseline_avg_us`, `baseline_ratio`, and `baseline_status`")
            && normalized_docs.contains("`baseline_status=missing`")
            && normalized_docs.contains("`radiant_perf_summary`")
            && normalized_docs.contains("`baseline_matched`, `baseline_missing`")
            && normalized_docs.contains("`baseline_faster`, `baseline_similar`, and `baseline_slower`")
            && normalized_docs.contains("incomplete trend artifacts are visible")
            && normalized_docs.contains("portable pass/fail gate by default")
            && normalized_docs.contains("`--fail-on-baseline-regression`")
            && normalized_docs.contains("exits with status `1`")
            && normalized_docs.contains("`baseline_status=slower`")
            && normalized_docs.contains("target-area category and default iteration count")
            && normalized_docs.contains("Layout scenarios:")
            && normalized_docs.contains("Application projection scenarios:")
            && normalized_docs.contains("Runtime surface scenarios:")
            && normalized_docs.contains("Resource lifecycle scenarios:")
            && normalized_docs.contains("Text scenarios:")
            && normalized_docs.contains("GPU data and surface scenarios:"),
        "docs/API.md should describe how to run and interpret the perf harness"
    );
}
