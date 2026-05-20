//! Filtering, listing, and timing for performance harness scenarios.

use std::{
    collections::BTreeMap,
    env, fs,
    path::PathBuf,
    time::{Duration, Instant},
};

const RUN_ALL_IN_DEBUG_ENV: &str = "RADIANT_PERF_RUN_ALL_IN_DEBUG";
const BASELINE_JSONL_ARG: &str = "--baseline-jsonl";
const WRITE_BASELINE_JSONL_ARG: &str = "--write-baseline-jsonl";
const FAIL_ON_BASELINE_REGRESSION_ARG: &str = "--fail-on-baseline-regression";
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
    output_format: OutputFormat,
    baseline: Option<BaselineSet>,
    baseline_output: Option<BaselineOutput>,
    baseline_summary: BaselineSummary,
    fail_on_baseline_regression: bool,
    matched: usize,
}

impl ScenarioRunner {
    pub(super) fn new(
        filters: Vec<String>,
        output_format: OutputFormat,
        baseline: Option<BaselineSet>,
        baseline_output: Option<BaselineOutput>,
        fail_on_baseline_regression: bool,
    ) -> Self {
        Self {
            filters,
            output_format,
            baseline,
            baseline_output,
            baseline_summary: BaselineSummary::default(),
            fail_on_baseline_regression,
            matched: 0,
        }
    }

    pub(super) fn run_scenario<Build, Bench>(&mut self, name: &str, iterations: usize, build: Build)
    where
        Build: FnOnce() -> Bench,
        Bench: FnMut(),
    {
        if !scenario_matches_filters(name, &self.filters) {
            return;
        }
        self.matched += 1;
        let metric = run_scenario(
            name,
            iterations,
            build(),
            self.output_format,
            self.baseline.as_ref(),
        );
        self.baseline_summary.record(metric.comparison);
        if let Some(output) = &mut self.baseline_output {
            output.record(metric.baseline_jsonl);
        }
    }

    pub(super) fn finish(self) {
        if self.matched == 0 && !self.filters.is_empty() {
            eprintln!(
                "no radiant_perf scenarios matched filters: {:?}",
                self.filters
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
                self.baseline_summary.slower
            );
            std::process::exit(1);
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BaselineSet {
    metrics: BTreeMap<String, BaselineMetric>,
}

impl BaselineSet {
    fn from_jsonl_file(path: PathBuf) -> Result<Self, String> {
        let source = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let mut metrics = BTreeMap::new();
        for (line_index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let metric = BaselineMetric::from_json_line(trimmed).map_err(|err| {
                format!(
                    "failed to parse {}:{} as radiant_perf JSONL: {err}",
                    path.display(),
                    line_index + 1
                )
            })?;
            metrics.insert(metric.scenario.clone(), metric);
        }
        Ok(Self { metrics })
    }

    fn metric_for(&self, scenario: &str) -> Option<&BaselineMetric> {
        self.metrics.get(scenario)
    }
}

pub(super) struct BaselineOutput {
    path: PathBuf,
    lines: Vec<String>,
}

impl BaselineOutput {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            lines: Vec::new(),
        }
    }

    fn record(&mut self, line: String) {
        self.lines.push(line);
    }

    fn write(self) -> Result<(), String> {
        let mut contents = self.lines.join("\n");
        if !contents.is_empty() {
            contents.push('\n');
        }
        fs::write(&self.path, contents)
            .map_err(|err| format!("failed to write {}: {err}", self.path.display()))
    }
}

#[derive(Clone, Debug)]
struct BaselineMetric {
    scenario: String,
    avg_us: f64,
}

impl BaselineMetric {
    fn from_json_line(line: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(line).map_err(|err| err.to_string())?;
        if value.get("type").and_then(serde_json::Value::as_str) != Some("radiant_perf") {
            return Err(String::from("expected type=\"radiant_perf\""));
        }
        let scenario = value
            .get("scenario")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| String::from("missing string field `scenario`"))?
            .to_owned();
        let avg_us = value
            .get("avg_us")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| String::from("missing numeric field `avg_us`"))?;
        if !avg_us.is_finite() || avg_us <= 0.0 {
            return Err(String::from("field `avg_us` must be finite and positive"));
        }
        Ok(Self { scenario, avg_us })
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
        if !arg.starts_with('-') && !arg.is_empty() {
            filters.push(arg.clone());
        }
    }
    filters
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

pub(super) fn print_scenario_list(scenarios: &[ScenarioSpec]) {
    println!("radiant_perf scenarios:");
    for scenario in scenarios {
        println!(
            "{} category={} iterations={}",
            scenario.name, scenario.category, scenario.iterations
        );
    }
}

pub(super) fn should_skip_unfiltered_debug_run(filters: &[String]) -> bool {
    cfg!(debug_assertions) && filters.is_empty() && env::var_os(RUN_ALL_IN_DEBUG_ENV).is_none()
}

pub(super) fn print_unfiltered_debug_skip() {
    println!(
        "radiant_perf skipped unfiltered debug run; pass a scenario filter or set {RUN_ALL_IN_DEBUG_ENV}=1"
    );
}

fn scenario_matches_filters(name: &str, filters: &[String]) -> bool {
    filters.is_empty() || filters.iter().any(|filter| name.contains(filter))
}

fn run_scenario(
    name: &str,
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
    iterations: usize,
    elapsed: Duration,
    output_format: OutputFormat,
    baseline: Option<Option<&BaselineMetric>>,
) -> ScenarioMetric {
    let total_us = elapsed.as_micros();
    let avg_us = total_us as f64 / iterations.max(1) as f64;
    let comparison = baseline.map(|baseline| MetricComparison::new(avg_us, baseline));
    let baseline_jsonl = baseline_metric_json_line(name, iterations, total_us, avg_us);
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
                            "radiant_perf scenario={name} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_avg_us={baseline_avg_us:.3} baseline_ratio={ratio:.3} baseline_status={status}"
                        );
                    }
                    MetricComparison::Missing => {
                        println!(
                            "radiant_perf scenario={name} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_status=missing"
                        );
                    }
                }
            } else {
                println!(
                    "radiant_perf scenario={name} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}"
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
                            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_avg_us\":{baseline_avg_us:.3},\"baseline_ratio\":{ratio:.3},\"baseline_status\":\"{status}\"}}",
                            json_escape(name),
                            iterations,
                            total_us,
                            avg_us,
                        );
                    }
                    MetricComparison::Missing => {
                        println!(
                            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_status\":\"missing\"}}",
                            json_escape(name),
                            iterations,
                            total_us,
                            avg_us,
                        );
                    }
                }
            } else {
                println!(
                    "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}}}",
                    json_escape(name),
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

#[derive(Default)]
struct BaselineSummary {
    matched: usize,
    missing: usize,
    faster: usize,
    similar: usize,
    slower: usize,
}

impl BaselineSummary {
    fn record(&mut self, comparison: Option<MetricComparison>) {
        match comparison {
            Some(MetricComparison::Matched { status, .. }) => {
                self.matched += 1;
                match status {
                    "faster" => self.faster += 1,
                    "similar" => self.similar += 1,
                    "slower" => self.slower += 1,
                    _ => {}
                }
            }
            Some(MetricComparison::Missing) => {
                self.missing += 1;
            }
            None => {}
        }
    }

    fn print(&self, scenarios: usize, output_format: OutputFormat) {
        match output_format {
            OutputFormat::Text => {
                println!(
                    "radiant_perf_summary scenarios={scenarios} baseline_matched={} baseline_missing={} baseline_faster={} baseline_similar={} baseline_slower={}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
            OutputFormat::JsonLines => {
                println!(
                    "{{\"type\":\"radiant_perf_summary\",\"scenarios\":{scenarios},\"baseline_matched\":{},\"baseline_missing\":{},\"baseline_faster\":{},\"baseline_similar\":{},\"baseline_slower\":{}}}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
        }
    }

    fn has_regression(&self) -> bool {
        self.slower > 0
    }
}

#[derive(Clone, Copy)]
enum MetricComparison {
    Matched {
        baseline_avg_us: f64,
        ratio: f64,
        status: &'static str,
    },
    Missing,
}

impl MetricComparison {
    fn new(avg_us: f64, baseline: Option<&BaselineMetric>) -> Self {
        let Some(baseline) = baseline else {
            return Self::Missing;
        };
        let baseline_avg_us = baseline.avg_us;
        let ratio = avg_us / baseline_avg_us;
        let status = if ratio > 1.05 {
            "slower"
        } else if ratio < 0.95 {
            "faster"
        } else {
            "similar"
        };
        Self::Matched {
            baseline_avg_us,
            ratio,
            status,
        }
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

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn baseline_metric_json_line(name: &str, iterations: usize, total_us: u128, avg_us: f64) -> String {
    format!(
        "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}}}",
        json_escape(name),
        iterations,
        total_us,
        avg_us
    )
}
