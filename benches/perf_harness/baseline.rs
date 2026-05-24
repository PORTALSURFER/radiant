use std::{collections::BTreeMap, fs, path::PathBuf};

mod format;
mod summary;

pub(super) use format::{baseline_metric_json_line, json_escape};
pub(super) use summary::BaselineSummary;

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
