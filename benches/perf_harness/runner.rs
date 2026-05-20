//! Filtering, listing, and timing for performance harness scenarios.

use std::{
    env,
    time::{Duration, Instant},
};

const RUN_ALL_IN_DEBUG_ENV: &str = "RADIANT_PERF_RUN_ALL_IN_DEBUG";
const JSONL_ARG: &str = "--jsonl";
const LIST_ARG: &str = "--list";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OutputFormat {
    Text,
    JsonLines,
}

pub(super) struct ScenarioRunner {
    filters: Vec<String>,
    output_format: OutputFormat,
    matched: usize,
}

impl ScenarioRunner {
    pub(super) fn new(filters: Vec<String>, output_format: OutputFormat) -> Self {
        Self {
            filters,
            output_format,
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
        run_scenario(name, iterations, build(), self.output_format);
    }

    pub(super) fn finish(self) {
        if self.matched == 0 && !self.filters.is_empty() {
            eprintln!(
                "no radiant_perf scenarios matched filters: {:?}",
                self.filters
            );
            std::process::exit(2);
        }
    }
}

pub(super) fn scenario_filters_from_args(args: &[String]) -> Vec<String> {
    args.iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-') && !arg.is_empty())
        .cloned()
        .collect()
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

pub(super) fn print_scenario_list(scenarios: &[&str]) {
    println!("radiant_perf scenarios:");
    for scenario in scenarios {
        println!("{scenario}");
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
) {
    bench();
    let started = Instant::now();
    for _ in 0..iterations {
        bench();
    }
    print_metric(name, iterations, started.elapsed(), output_format);
}

fn print_metric(name: &str, iterations: usize, elapsed: Duration, output_format: OutputFormat) {
    let total_us = elapsed.as_micros();
    let avg_us = total_us as f64 / iterations.max(1) as f64;
    match output_format {
        OutputFormat::Text => {
            println!(
                "radiant_perf scenario={name} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}"
            );
        }
        OutputFormat::JsonLines => {
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
