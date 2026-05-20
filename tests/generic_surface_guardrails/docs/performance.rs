use super::*;

#[test]
fn performance_harness_is_registered_and_documented() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(manifest_dir.join("Cargo.toml"))
        .expect("Radiant Cargo.toml should be readable");
    let bench = fs::read_to_string(manifest_dir.join("benches/perf_harness.rs"))
        .expect("perf_harness bench should be readable");
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
        "runtime_virtualized_nested_scroll_hover_10k",
        "runtime_refresh_large_tree",
        "runtime_resize_large_tree",
        "runtime_command_flattening_512",
        "runtime_command_drain_1k",
        "runtime_nested_command_drain_1k",
        "gpu_signal_summary",
        "gpu_surface_projection",
    ];
    for scenario in perf_scenarios {
        assert!(
            bench.contains(scenario),
            "perf_harness should include `{scenario}`"
        );
        let scenario_literal = format!("\"{scenario}\"");
        assert_eq!(
            bench.matches(&scenario_literal).count(),
            1,
            "perf_harness should register `{scenario}` in a single shared scenario catalog"
        );
        assert!(
            docs.contains(scenario),
            "docs/API.md should document perf scenario `{scenario}`"
        );
    }
    assert!(
        bench.contains("radiant_perf scenario="),
        "perf_harness should print parseable metric lines"
    );
    assert!(
        bench.contains("--list") && bench.contains("radiant_perf scenarios:"),
        "perf_harness should expose a cheap scenario-listing mode"
    );
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_docs.contains("cargo bench --bench perf_harness")
            && normalized_docs.contains("cargo bench --bench perf_harness -- --list")
            && normalized_docs
                .contains("does not enforce machine-dependent pass/fail timing thresholds"),
        "docs/API.md should describe how to run and interpret the perf harness"
    );
}
