//! Filtering, listing, and timing for performance harness scenarios.

mod args;
mod baseline;
#[path = "runner/metrics.rs"]
mod metrics;

use std::{env, time::Instant};

pub(super) use args::{
    baseline_from_args, baseline_output_from_args, category_filters_from_args,
    fail_on_baseline_regression_from_args, fail_on_missing_baseline_from_args,
    group_filters_from_args, output_format_from_args, scenario_filters_from_args,
    scenario_list_requested,
};
use baseline::{BaselineOutput, BaselineSet, BaselineSummary};
pub(crate) use metrics::ScenarioCounters;
use metrics::ScenarioMetric;

const RUN_ALL_IN_DEBUG_ENV: &str = args::RUN_ALL_IN_DEBUG_ENV;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OutputFormat {
    Text,
    JsonLines,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ScenarioSpec {
    pub(super) name: &'static str,
    pub(super) category: &'static str,
    pub(super) group: &'static str,
    pub(super) counters: &'static [&'static str],
    pub(super) iterations: usize,
}

impl ScenarioSpec {
    pub(super) const fn new(
        name: &'static str,
        category: &'static str,
        group: &'static str,
        counters: &'static [&'static str],
        iterations: usize,
    ) -> Self {
        Self {
            name,
            category,
            group,
            counters,
            iterations,
        }
    }
}

pub(super) struct ScenarioRunner {
    filters: Vec<String>,
    category_filters: Vec<String>,
    group_filters: Vec<String>,
    output_format: OutputFormat,
    baseline: Option<BaselineSet>,
    baseline_output: Option<BaselineOutput>,
    baseline_summary: BaselineSummary,
    fail_on_baseline_regression: bool,
    fail_on_missing_baseline: bool,
    matched: usize,
}

pub(super) struct ScenarioRunnerConfig {
    pub(super) filters: Vec<String>,
    pub(super) category_filters: Vec<String>,
    pub(super) group_filters: Vec<String>,
    pub(super) output_format: OutputFormat,
    pub(super) baseline: Option<BaselineSet>,
    pub(super) baseline_output: Option<BaselineOutput>,
    pub(super) fail_on_baseline_regression: bool,
    pub(super) fail_on_missing_baseline: bool,
}

impl ScenarioRunner {
    pub(super) fn new(config: ScenarioRunnerConfig) -> Self {
        Self {
            filters: config.filters,
            category_filters: config.category_filters,
            group_filters: config.group_filters,
            output_format: config.output_format,
            baseline: config.baseline,
            baseline_output: config.baseline_output,
            baseline_summary: BaselineSummary::default(),
            fail_on_baseline_regression: config.fail_on_baseline_regression,
            fail_on_missing_baseline: config.fail_on_missing_baseline,
            matched: 0,
        }
    }

    pub(super) fn run_scenario<Build, Bench, Sample>(
        &mut self,
        name: &str,
        category: &str,
        group: &str,
        iterations: usize,
        build: Build,
    ) where
        Build: FnOnce() -> Bench,
        Bench: FnMut() -> Sample,
        Sample: Into<ScenarioCounters>,
    {
        if !scenario_matches_filters(
            name,
            category,
            group,
            &self.filters,
            &self.category_filters,
            &self.group_filters,
        ) {
            return;
        }
        self.matched += 1;
        let metric = run_scenario(
            name,
            category,
            group,
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
        if self.matched == 0
            && (!self.filters.is_empty()
                || !self.category_filters.is_empty()
                || !self.group_filters.is_empty())
        {
            eprintln!(
                "no radiant_perf scenarios matched filters: {:?} categories: {:?} groups: {:?}",
                self.filters, self.category_filters, self.group_filters
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

pub(super) fn print_scenario_list(scenarios: &[ScenarioSpec]) {
    println!("radiant_perf scenarios:");
    for scenario in scenarios {
        println!(
            "{} category={} group={} iterations={} counters={}",
            scenario.name,
            scenario.category,
            scenario.group,
            scenario.iterations,
            if scenario.counters.is_empty() {
                "none".to_owned()
            } else {
                scenario.counters.join(",")
            }
        );
    }
}

pub(super) fn should_skip_unfiltered_debug_run(
    filters: &[String],
    category_filters: &[String],
    group_filters: &[String],
) -> bool {
    cfg!(debug_assertions)
        && filters.is_empty()
        && category_filters.is_empty()
        && group_filters.is_empty()
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
    group: &str,
    filters: &[String],
    category_filters: &[String],
    group_filters: &[String],
) -> bool {
    let name_matches = filters.is_empty() || filters.iter().any(|filter| name.contains(filter));
    let category_matches = category_filters.is_empty()
        || category_filters
            .iter()
            .any(|filter| category.contains(filter));
    let group_matches =
        group_filters.is_empty() || group_filters.iter().any(|filter| group.contains(filter));
    name_matches && category_matches && group_matches
}

fn run_scenario<Bench, Sample>(
    name: &str,
    category: &str,
    group: &str,
    iterations: usize,
    mut bench: Bench,
    output_format: OutputFormat,
    baseline: Option<&BaselineSet>,
) -> ScenarioMetric
where
    Bench: FnMut() -> Sample,
    Sample: Into<ScenarioCounters>,
{
    bench();
    let started = Instant::now();
    let mut counters = ScenarioCounters::default();
    for _ in 0..iterations {
        counters.add(bench().into());
    }
    ScenarioMetric::print(
        name,
        category,
        group,
        iterations,
        started.elapsed(),
        counters,
        output_format,
        baseline.map(|baseline| baseline.metric_for(name)),
    )
}
