use std::{collections::BTreeMap, fs, path::PathBuf};

use super::OutputFormat;

#[derive(Clone, Debug)]
pub(crate) struct BaselineSet {
    metrics: BTreeMap<String, BaselineMetric>,
}

impl BaselineSet {
    pub(super) fn from_jsonl_file(path: PathBuf) -> Result<Self, String> {
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

    pub(super) fn metric_for(&self, scenario: &str) -> Option<&BaselineMetric> {
        self.metrics.get(scenario)
    }
}

pub(crate) struct BaselineOutput {
    path: PathBuf,
    lines: Vec<String>,
}

impl BaselineOutput {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lines: Vec::new(),
        }
    }

    pub(super) fn record(&mut self, line: String) {
        self.lines.push(line);
    }

    pub(super) fn write(self) -> Result<(), String> {
        let mut contents = self.lines.join("\n");
        if !contents.is_empty() {
            contents.push('\n');
        }
        fs::write(&self.path, contents)
            .map_err(|err| format!("failed to write {}: {err}", self.path.display()))
    }
}

#[derive(Clone, Debug)]
pub(super) struct BaselineMetric {
    scenario: String,
    pub(super) avg_us: f64,
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

#[derive(Default)]
pub(super) struct BaselineSummary {
    categories: BTreeMap<String, BaselineSummaryCounts>,
    total: BaselineSummaryCounts,
}

#[derive(Clone, Copy, Default)]
struct BaselineSummaryCounts {
    matched: usize,
    missing: usize,
    faster: usize,
    similar: usize,
    slower: usize,
}

impl BaselineSummary {
    pub(super) fn record(&mut self, category: &str, comparison: Option<MetricComparison>) {
        self.total.record(comparison);
        self.categories
            .entry(category.to_owned())
            .or_default()
            .record(comparison);
    }

    pub(super) fn print(&self, scenarios: usize, output_format: OutputFormat) {
        self.total.print_total(scenarios, output_format);
        for (category, counts) in &self.categories {
            counts.print_category(category, output_format);
        }
    }

    pub(super) fn has_regression(&self) -> bool {
        self.total.slower > 0
    }

    pub(super) fn has_missing_baseline(&self) -> bool {
        self.total.missing > 0
    }

    pub(super) fn slower(&self) -> usize {
        self.total.slower
    }

    pub(super) fn missing(&self) -> usize {
        self.total.missing
    }
}

impl BaselineSummaryCounts {
    fn scenarios(&self) -> usize {
        self.matched + self.missing
    }

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

    fn print_total(&self, scenarios: usize, output_format: OutputFormat) {
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

    fn print_category(&self, category: &str, output_format: OutputFormat) {
        let scenarios = self.scenarios();
        match output_format {
            OutputFormat::Text => {
                println!(
                    "radiant_perf_category_summary category={category} scenarios={scenarios} baseline_matched={} baseline_missing={} baseline_faster={} baseline_similar={} baseline_slower={}",
                    self.matched, self.missing, self.faster, self.similar, self.slower
                );
            }
            OutputFormat::JsonLines => {
                println!(
                    "{{\"type\":\"radiant_perf_category_summary\",\"category\":\"{}\",\"scenarios\":{scenarios},\"baseline_matched\":{},\"baseline_missing\":{},\"baseline_faster\":{},\"baseline_similar\":{},\"baseline_slower\":{}}}",
                    json_escape(category),
                    self.matched,
                    self.missing,
                    self.faster,
                    self.similar,
                    self.slower
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum MetricComparison {
    Matched {
        baseline_avg_us: f64,
        ratio: f64,
        status: &'static str,
    },
    Missing,
}

impl MetricComparison {
    pub(super) fn new(avg_us: f64, baseline: Option<&BaselineMetric>) -> Self {
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

pub(super) fn json_escape(value: &str) -> String {
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

pub(super) fn baseline_metric_json_line(
    name: &str,
    category: &str,
    iterations: usize,
    total_us: u128,
    avg_us: f64,
) -> String {
    format!(
        "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}}}",
        json_escape(name),
        json_escape(category),
        iterations,
        total_us,
        avg_us
    )
}
