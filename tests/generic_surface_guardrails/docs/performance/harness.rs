use super::support::{PERF_SCENARIOS, read_project_file};

#[test]
fn performance_harness_is_registered_and_cataloged() {
    let manifest = read_project_file("Cargo.toml");
    let bench = read_project_file("benches/perf_harness.rs");
    let catalog = read_project_file("benches/perf_harness/catalog.rs");
    let docs = read_project_file("docs/API.md");

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
    for scenario in PERF_SCENARIOS {
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
}

#[test]
fn performance_harness_runner_exposes_metrics_baselines_and_listing() {
    let runner = read_project_file("benches/perf_harness/runner.rs");
    let metrics = read_project_file("benches/perf_harness/runner/metrics.rs");
    let args = read_project_file("benches/perf_harness/args.rs");
    let baseline = read_project_file("benches/perf_harness/baseline.rs");
    let baseline_summary = read_project_file("benches/perf_harness/baseline/summary.rs");
    let baseline_format = read_project_file("benches/perf_harness/baseline/format.rs");
    let catalog = read_project_file("benches/perf_harness/catalog.rs");
    let runner_contract = format!("{runner}\n{metrics}");

    assert!(
        runner_contract.contains("radiant_perf scenario="),
        "perf_harness runner should print parseable metric lines"
    );
    assert!(
        args.contains("--jsonl")
            && args.contains("--baseline-jsonl")
            && args.contains("--write-baseline-jsonl")
            && args.contains("--category")
            && args.contains("--fail-on-baseline-regression")
            && args.contains("--fail-on-missing-baseline")
            && runner_contract.contains("OutputFormat::JsonLines")
            && runner.contains("ScenarioRunnerConfig")
            && runner.contains("BaselineSet")
            && runner.contains("BaselineOutput")
            && runner.contains("baseline_output_from_args")
            && runner.contains("fail_on_baseline_regression")
            && runner.contains("fail_on_missing_baseline")
            && runner.contains("mod args;")
            && runner.contains("mod baseline;")
            && !runner.contains("fn value_after_arg")
            && args.contains("fn value_after_arg")
            && !runner.contains("struct BaselineSummaryCounts")
            && !baseline.contains("struct BaselineSummaryCounts")
            && baseline.contains("mod summary;")
            && baseline.contains("mod format;")
            && baseline_summary.contains("struct BaselineSummaryCounts")
            && baseline_format.contains("fn baseline_metric_json_line")
            && runner.contains("has_regression")
            && runner.contains("has_missing_baseline")
            && metrics.contains("baseline_metric_json_line")
            && runner.contains("std::process::exit(1)")
            && metrics.contains("\\\"type\\\":\\\"radiant_perf\\\"")
            && metrics.contains("category={category}")
            && metrics.contains("\\\"category\\\":\\\"{}\\\"")
            && metrics.contains("\\\"total_us\\\"")
            && metrics.contains("\\\"avg_us\\\"")
            && metrics.contains("\\\"baseline_avg_us\\\"")
            && metrics.contains("\\\"baseline_ratio\\\"")
            && metrics.contains("\\\"baseline_status\\\"")
            && runner.contains("BaselineSummary")
            && baseline_summary.contains("radiant_perf_summary")
            && baseline_summary.contains("radiant_perf_category_summary")
            && baseline_summary.contains("\\\"type\\\":\\\"radiant_perf_category_summary\\\"")
            && baseline_summary.contains("baseline_missing")
            && baseline_summary.contains("\\\"baseline_slower\\\"")
            && baseline_summary.contains("MetricComparison::Missing")
            && metrics.contains("baseline_status=missing")
            && metrics.contains("\\\"baseline_status\\\":\\\"missing\\\""),
        "perf_harness runner should expose JSON-lines metrics with target-area categories while delegating argument parsing and baseline summaries"
    );
    assert!(
        args.contains("--list") && runner.contains("radiant_perf scenarios:"),
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
            && runner.contains("category_filters_from_args")
            && args.contains("category_filters_from_args")
            && runner.contains("category_filters")
            && runner.contains("category={}")
            && runner.contains("iterations={}"),
        "perf_harness scenario listing and filters should expose target-area categories and iteration counts"
    );
}

#[test]
fn performance_harness_exercises_custom_shader_projection() {
    let bench = read_project_file("benches/perf_harness.rs");

    assert!(
        bench.contains("bench_gpu_custom_shader_projection")
            && bench.contains("GpuShaderSurfaceDescriptor::new")
            && bench.contains("GpuSurfaceContent::CustomShader"),
        "perf_harness should exercise backend-neutral custom shader GPU surface projection"
    );
}
