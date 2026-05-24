//! Metric emission for performance harness scenarios.

use std::time::Duration;

use super::{
    OutputFormat,
    baseline::{BaselineMetric, MetricComparison, baseline_metric_json_line, json_escape},
};

pub(super) struct ScenarioMetric {
    pub(super) comparison: Option<MetricComparison>,
    pub(super) baseline_jsonl: String,
}

impl ScenarioMetric {
    pub(super) fn print(
        name: &str,
        category: &str,
        iterations: usize,
        elapsed: Duration,
        output_format: OutputFormat,
        baseline: Option<Option<&BaselineMetric>>,
    ) -> Self {
        let total_us = elapsed.as_micros();
        let avg_us = total_us as f64 / iterations.max(1) as f64;
        let comparison = baseline.map(|baseline| MetricComparison::new(avg_us, baseline));
        let baseline_jsonl =
            baseline_metric_json_line(name, category, iterations, total_us, avg_us);
        match output_format {
            OutputFormat::Text => {
                print_text_metric(name, category, iterations, total_us, avg_us, comparison)
            }
            OutputFormat::JsonLines => {
                print_json_metric(name, category, iterations, total_us, avg_us, comparison)
            }
        }
        Self {
            comparison,
            baseline_jsonl,
        }
    }
}

fn print_text_metric(
    name: &str,
    category: &str,
    iterations: usize,
    total_us: u128,
    avg_us: f64,
    comparison: Option<MetricComparison>,
) {
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => println!(
            "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_avg_us={baseline_avg_us:.3} baseline_ratio={ratio:.3} baseline_status={status}"
        ),
        Some(MetricComparison::Missing) => println!(
            "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} baseline_status=missing"
        ),
        None => println!(
            "radiant_perf scenario={name} category={category} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}"
        ),
    }
}

fn print_json_metric(
    name: &str,
    category: &str,
    iterations: usize,
    total_us: u128,
    avg_us: f64,
    comparison: Option<MetricComparison>,
) {
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_avg_us\":{baseline_avg_us:.3},\"baseline_ratio\":{ratio:.3},\"baseline_status\":\"{status}\"}}",
            json_escape(name),
            json_escape(category),
            iterations,
            total_us,
            avg_us,
        ),
        Some(MetricComparison::Missing) => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"baseline_status\":\"missing\"}}",
            json_escape(name),
            json_escape(category),
            iterations,
            total_us,
            avg_us,
        ),
        None => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}}}",
            json_escape(name),
            json_escape(category),
            iterations,
            total_us,
            avg_us
        ),
    }
}
