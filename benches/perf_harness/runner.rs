//! Filtering, listing, and timing for performance harness scenarios.

mod baseline;

use std::{
    env,
    path::PathBuf,
    time::{Duration, Instant},
};

use baseline::{
    BaselineMetric, BaselineOutput, BaselineSet, BaselineSummary, MetricComparison,
    baseline_metric_json_line, json_escape,
};

const RUN_ALL_IN_DEBUG_ENV: &str = "RADIANT_PERF_RUN_ALL_IN_DEBUG";
const BASELINE_JSONL_ARG: &str = "--baseline-jsonl";
const WRITE_BASELINE_JSONL_ARG: &str = "--write-baseline-jsonl";
const CATEGORY_ARG: &str = "--category";
const FAIL_ON_BASELINE_REGRESSION_ARG: &str = "--fail-on-baseline-regression";
const FAIL_ON_MISSING_BASELINE_ARG: &str = "--fail-on-missing-baseline";
const JSONL_ARG: &str = "--jsonl";
const LIST_ARG: &str = "--list";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OutputFormat {
    Text,
    JsonLines,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ScenarioSpec {
    pub(super) name: &'static str,
    pub(super) category: &'static str,
    pub(super) iterations: usize,
}

impl ScenarioSpec {
    pub(super) const fn new(name: &'static str, category: &'static str, iterations: usize) -> Self {
        Self {
            name,
            category,
            iterations,
        }
    }
}

pub(super) struct ScenarioRunner {
    filters: Vec<String>,
    category_filters: Vec<String>,
    output_format: OutputFormat,
    baseline: Option<BaselineSet>,
    baseline_output: Option<BaselineOutput>,
    baseline_summary: BaselineSummary,
    fail_on_baseline_regression: bool,
    fail_on_missing_baseline: bool,
    matched: usize,
}

impl ScenarioRunner {
    pub(super) fn new(
        filters: Vec<String>,
        category_filters: Vec<String>,
        output_format: OutputFormat,
        baseline: Option<BaselineSet>,
        baseline_output: Option<BaselineOutput>,
        fail_on_baseline_regression: bool,
        fail_on_missing_baseline: bool,
    ) -> Self {
        Self {
            filters,
            category_filters,
            output_format,
            baseline,
            baseline_output,
            baseline_summary: BaselineSummary::default(),
            fail_on_baseline_regression,
            fail_on_missing_baseline,
            matched: 0,
        }
    }

    pub(super) fn run_scenario<Build, Bench>(
        &mut self,
        name: &str,
        category: &str,
        iterations: usize,
        build: Build,
    ) where
        Build: FnOnce() -> Bench,
        Bench: FnMut(),
    {
        if !scenario_matches_filters(name, category, &self.filters, &self.category_filters) {
            return;
        }
        self.matched += 1;
        let metric = run_scenario(
            name,
            category,
            iterations,
            build(),
            self.output_format,
            self.baseline.as_ref(),
        );
        self.baseline_summary.record(category, metric.comparison);
        if let Some(output) = &mut self.baseline_output {
            output.record(metric.baseline_jsonl);
        }
    }

    pub(super) fn finish(self) {
        if self.matched == 0 && (!self.filters.is_empty() || !self.category_filters.is_empty()) {
            eprintln!(
                "no radiant_perf scenarios matched filters: {:?} categories: {:?}",
                self.filters, self.category_filters
            );
            std::process::exit(2);
        }
        if self.baseline.is_some() && self.matched > 0 {
            self.baseline_summary
                .print(self.matched, self.output_format);
        }
        if let Some(output) = self.baseline_output
            && let Err(err) = output.write()
        {
            eprintln!("radiant_perf baseline error: {err}");
            std::process::exit(2);
        }
        if self.fail_on_baseline_regression && self.baseline_summary.has_regression() {
            eprintln!(
                "radiant_perf regression gate failed: {} slower scenario(s)",
                self.baseline_summary.slower()
            );
            std::process::exit(1);
        }
        if self.fail_on_missing_baseline && self.baseline_summary.has_missing_baseline() {
            eprintln!(
                "radiant_perf baseline coverage gate failed: {} missing scenario baseline(s)",
                self.baseline_summary.missing()
            );
            std::process::exit(1);
        }
    }
}

pub(super) fn scenario_filters_from_args(args: &[String]) -> Vec<String> {
    let mut filters = Vec::new();
    let mut skip_next = false;
    for arg in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == BASELINE_JSONL_ARG {
            skip_next = true;
            continue;
        }
        if arg.starts_with(&format!("{BASELINE_JSONL_ARG}=")) {
            continue;
        }
        if arg == WRITE_BASELINE_JSONL_ARG {
            skip_next = true;
            continue;
        }
        if arg.starts_with(&format!("{WRITE_BASELINE_JSONL_ARG}=")) {
            continue;
        }
        if arg == CATEGORY_ARG {
            skip_next = true;
            continue;
        }
        if arg.starts_with(&format!("{CATEGORY_ARG}=")) {
            continue;
        }
        if !arg.starts_with('-') && !arg.is_empty() {
            filters.push(arg.clone());
        }
    }
    filters
}

pub(super) fn category_filters_from_args(args: &[String]) -> Vec<String> {
    values_after_arg(args, CATEGORY_ARG)
}

pub(super) fn scenario_list_requested(args: &[String]) -> bool {
    args.iter().skip(1).any(|arg| arg == LIST_ARG)
}

pub(super) fn output_format_from_args(args: &[String]) -> OutputFormat {
    if args.iter().skip(1).any(|arg| arg == JSONL_ARG) {
        OutputFormat::JsonLines
    } else {
        OutputFormat::Text
    }
}

pub(super) fn baseline_from_args(args: &[String]) -> Option<BaselineSet> {
    let path = match value_after_arg(args, BASELINE_JSONL_ARG) {
        Some(path) => path,
        None if args.iter().skip(1).any(|arg| arg == BASELINE_JSONL_ARG) => {
            eprintln!("radiant_perf baseline error: --baseline-jsonl requires a path");
            std::process::exit(2);
        }
        None => return None,
    };
    match BaselineSet::from_jsonl_file(PathBuf::from(path)) {
        Ok(baseline) => Some(baseline),
        Err(err) => {
            eprintln!("radiant_perf baseline error: {err}");
            std::process::exit(2);
        }
    }
}

pub(super) fn baseline_output_from_args(args: &[String]) -> Option<BaselineOutput> {
    let path = match value_after_arg(args, WRITE_BASELINE_JSONL_ARG) {
        Some(path) => path,
        None if args
            .iter()
            .skip(1)
            .any(|arg| arg == WRITE_BASELINE_JSONL_ARG) =>
        {
            eprintln!("radiant_perf baseline error: --write-baseline-jsonl requires a path");
            std::process::exit(2);
        }
        None => return None,
    };
    Some(BaselineOutput::new(PathBuf::from(path)))
}

pub(super) fn fail_on_baseline_regression_from_args(args: &[String]) -> bool {
    let fail_on_regression = args
        .iter()
        .skip(1)
        .any(|arg| arg == FAIL_ON_BASELINE_REGRESSION_ARG);
    if fail_on_regression && value_after_arg(args, BASELINE_JSONL_ARG).is_none() {
        eprintln!(
            "radiant_perf baseline error: {FAIL_ON_BASELINE_REGRESSION_ARG} requires --baseline-jsonl"
        );
        std::process::exit(2);
    }
    fail_on_regression
}

pub(super) fn fail_on_missing_baseline_from_args(args: &[String]) -> bool {
    let fail_on_missing = args
        .iter()
        .skip(1)
        .any(|arg| arg == FAIL_ON_MISSING_BASELINE_ARG);
    if fail_on_missing && value_after_arg(args, BASELINE_JSONL_ARG).is_none() {
        eprintln!(
            "radiant_perf baseline error: {FAIL_ON_MISSING_BASELINE_ARG} requires --baseline-jsonl"
        );
        std::process::exit(2);
    }
    fail_on_missing
}

pub(super) fn print_scenario_list(scenarios: &[ScenarioSpec]) {
    println!("radiant_perf scenarios:");
    for scenario in scenarios {
        println!(
            "{} category={} iterations={}",
            scenario.name, scenario.category, scenario.iterations
        );
    }
}

pub(super) fn should_skip_unfiltered_debug_run(
    filters: &[String],
    category_filters: &[String],
) -> bool {
    cfg!(debug_assertions)
        && filters.is_empty()
        && category_filters.is_empty()
        && env::var_os(RUN_ALL_IN_DEBUG_ENV).is_none()
}

pub(super) fn print_unfiltered_debug_skip() {
    println!(
        "radiant_perf skipped unfiltered debug run; pass a scenario filter or set {RUN_ALL_IN_DEBUG_ENV}=1"
    );
}

fn scenario_matches_filters(
    name: &str,
    category: &str,
    filters: &[String],
    category_filters: &[String],
) -> bool {
    let name_matches = filters.is_empty() || filters.iter().any(|filter| name.contains(filter));
    let category_matches = category_filters.is_empty()
        || category_filters
            .iter()
            .any(|filter| category.contains(filter));
    name_matches && category_matches
}

fn run_scenario(
    name: &str,
    category: &str,
    iterations: usize,
    mut bench: impl FnMut(),
    output_format: OutputFormat,
    baseline: Option<&BaselineSet>,
) -> ScenarioMetric {
    bench();
    let started = Instant::now();
    for _ in 0..iterations {
        bench();
    }
    print_metric(
        name,
        category,
        iterations,
        started.elapsed(),
        output_format,
        baseline.map(|baseline| baseline.metric_for(name)),
    )
}

struct ScenarioMetric {
    comparison: Option<MetricComparison>,
    baseline_jsonl: String,
}

fn print_metric(
    name: &str,
    category: &str,
    iterations: usize,
    elapsed: Duration,
    output_format: OutputFormat,
    baseline: Option<Option<&BaselineMetric>>,
) -> ScenarioMetric {
    let total_us = elapsed.as_micros();
    let avg_us = total_us as f64 / iterations.max(1) as f64;
    let comparison = baseline.map(|baseline| MetricComparison::new(avg_us, baseline));
    let baseline_jsonl = baseline_metric_json_line(name, category, iterations, total_us, avg_us);
    match output_format {
        OutputFormat::Text => {
            if let Some(comparison) = comparison {
                match comparison {
                    MetricComparison::Matched {
                        baseline_avg_us,
                        ratio,
                        status,
                    } => {
                        println!(
                            "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_avg_us={baseline_avg_us:.3} baseline_ratio={ratio:.3} baseline_status={status}"
                        );
                    }
                    MetricComparison::Missing => {
                        println!(
                            "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_status=missing"
                        );
                    }
                }
            } else {
                println!(
                    "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}"
                );
            }
        }
        OutputFormat::JsonLines => {
            if let Some(comparison) = comparison {
                match comparison {
                    MetricComparison::Matched {
                        baseline_avg_us,
                        ratio,
                        status,
                    } => {
                        println!(
                            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_avg_us\":{baseline_avg_us:.3},\"baseline_ratio\":{ratio:.3},\"baseline_status\":\"{status}\"}}",
                            json_escape(name),
                            json_escape(category),
                            iterations,
                            total_us,
                            avg_us,
                        );
                    }
                    MetricComparison::Missing => {
                        println!(
                            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_status\":\"missing\"}}",
                            json_escape(name),
                            json_escape(category),
                            iterations,
                            total_us,
                            avg_us,
                        );
                    }
                }
            } else {
                println!(
                    "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}}}",
                    json_escape(name),
                    json_escape(category),
                    iterations,
                    total_us,
                    avg_us
                );
            }
        }
    }
    ScenarioMetric {
        comparison,
        baseline_jsonl,
    }
}

fn value_after_arg(args: &[String], name: &str) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            return Some(value.to_owned());
        }
    }
    None
}

fn values_after_arg(args: &[String], name: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == name {
            if let Some(value) = iter.next() {
                values.push(value.clone());
            }
            continue;
        }
        if let Some(value) = arg.strip_prefix(&format!("{name}=")) {
            values.push(value.to_owned());
        }
    }
    values
}
