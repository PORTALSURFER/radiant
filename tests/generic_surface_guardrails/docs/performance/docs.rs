use super::support::{normalized_words, read_project_file};

#[test]
fn performance_docs_describe_harness_commands_and_output_contract() {
    let docs = read_project_file("docs/API.md");
    let normalized_docs = normalized_words(&docs);

    for required in [
        "cargo bench --bench perf_harness",
        "cargo bench --bench perf_harness -- --list",
        "cargo bench --bench perf_harness -- --category runtime_virtualized --jsonl",
        "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl",
        "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --write-baseline-jsonl",
        "cargo bench --bench perf_harness runtime_virtualized_list_hover -- --jsonl --baseline-jsonl",
        "Each JSON line includes `type`, `scenario`, `category`, `group`, `iterations`, `total_us`, `avg_us`, and finite nearest-rank `p50_us`, `p95_us`, and `p99_us`, plus any scenario-owned counters such as",
        "`p50_us`, `p95_us`, and `p99_us` are finite nearest-rank percentiles",
        "p50_us <= p95_us <= p99_us",
        "`scene_rebuild_count`, `static_rebuild_count`, `paint_only_count`",
        "`surface_refresh_count`",
        "`relayout_count`, `dirty_mark_count`, `overlay_paint_count`",
        "`overlay_rebuild_count`",
        "`text_cache_hit_count`",
        "`retained_surface_cache_hit_count`",
        "`frame_cadence_due_count`, `frame_cadence_wait_count`",
        "`text_storage_allocation_count`",
        "`encoded_paint_primitive_count`",
        "`scene_append_count`",
        "Capture a machine-local baseline artifact directly",
        "`baseline_avg_us`, `baseline_ratio`, and `baseline_status`",
        "`baseline_status=missing`",
        "`radiant_perf_summary`",
        "`radiant_perf_category_summary`",
        "`baseline_faster`, `baseline_similar`, and `baseline_slower`",
        "one `radiant_perf_category_summary` line per target-area category",
        "`--fail-on-baseline-regression`",
        "`--fail-on-missing-baseline`",
        "Metric lines and list output both include each scenario's target-area category, blessed review group, default iteration count, and advertised counters",
        "cargo bench --bench perf_harness -- --group pointer_motion --jsonl",
        "cargo bench --manifest-path vendor/radiant/Cargo.toml --bench perf_harness -- --group pointer_motion --jsonl",
        "`pointer_motion`",
        "`virtual_lists`",
        "`scene_cache`",
        "`text_layout`",
        "`retained_gpu_surfaces`",
        "`frame_cadence`",
        "cargo bench --bench perf_harness -- --group frame_cadence --jsonl --baseline-jsonl",
        "cargo bench --bench perf_harness runtime_arrangement_shell -- --jsonl",
        "runtime_arrangement_shell_frame_refresh",
        "runtime_arrangement_shell_structural_toggle",
        "runtime_arrangement_shell_hover_paint_only",
        "`standalone_gui`",
        "legacy baseline JSONL remains readable",
        "bounded batches avoid a clock read around every tiny iteration",
        "The local validation lane runs formatting",
        "no-default-features library checks for the documented Linux and macOS targets",
        "focused baseline round trip proves the JSONL capture/comparison path and missing baseline failure mode",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should describe perf harness detail `{required}`"
        );
    }
}

#[test]
fn performance_docs_group_scenarios_by_target_area() {
    let docs = read_project_file("docs/API.md");
    let normalized_docs = normalized_words(&docs);

    for required in [
        "Layout scenarios:",
        "Application projection scenarios:",
        "Runtime surface scenarios:",
        "Resource lifecycle scenarios:",
        "Text scenarios:",
        "GPU data and surface scenarios:",
        "Vello artifact strategy scenarios:",
        "Blessed high-risk groups:",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should group perf harness scenarios under `{required}`"
        );
    }
}
