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
    pub(super) _p50_us: Option<f64>,
    pub(super) _p95_us: Option<f64>,
    pub(super) _p99_us: Option<f64>,
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
        let p50_us = optional_percentile(&value, "p50_us")?;
        let p95_us = optional_percentile(&value, "p95_us")?;
        let p99_us = optional_percentile(&value, "p99_us")?;
        let percentile_count = [p50_us, p95_us, p99_us]
            .into_iter()
            .filter(Option::is_some)
            .count();
        if percentile_count != 0 && percentile_count != 3 {
            return Err(String::from(
                "fields `p50_us`, `p95_us`, and `p99_us` must be supplied together",
            ));
        }
        if let (Some(p50_us), Some(p95_us), Some(p99_us)) = (p50_us, p95_us, p99_us)
            && (p50_us > p95_us || p95_us > p99_us)
        {
            return Err(String::from(
                "fields `p50_us`, `p95_us`, and `p99_us` must be ordered",
            ));
        }
        Ok(Self {
            scenario,
            avg_us,
            _p50_us: p50_us,
            _p95_us: p95_us,
            _p99_us: p99_us,
        })
    }
}

fn optional_percentile(value: &serde_json::Value, field: &str) -> Result<Option<f64>, String> {
    let Some(value) = value.get(field) else {
        return Ok(None);
    };
    let value = value
        .as_f64()
        .ok_or_else(|| format!("field `{field}` must be numeric"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("field `{field}` must be finite and non-negative"));
    }
    Ok(Some(value))
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

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::BaselineMetric;

    #[test]
    fn legacy_baseline_without_percentiles_remains_readable() {
        let metric = BaselineMetric::from_json_line(
            r#"{"type":"radiant_perf","scenario":"legacy","avg_us":2.5}"#,
        )
        .expect("legacy baseline should parse");
        assert_eq!(metric.avg_us, 2.5);
        assert_eq!(metric._p50_us, None);
        assert_eq!(metric._p95_us, None);
        assert_eq!(metric._p99_us, None);
    }

    #[test]
    fn baseline_percentiles_require_ordered_complete_values() {
        let metric = BaselineMetric::from_json_line(
            r#"{"type":"radiant_perf","scenario":"new","avg_us":2.5,"p50_us":1.0,"p95_us":2.0,"p99_us":3.0}"#,
        )
        .expect("new baseline should parse");
        assert_eq!(metric._p50_us, Some(1.0));
        assert_eq!(metric._p95_us, Some(2.0));
        assert_eq!(metric._p99_us, Some(3.0));

        let unordered = BaselineMetric::from_json_line(
            r#"{"type":"radiant_perf","scenario":"bad","avg_us":2.5,"p50_us":3.0,"p95_us":2.0,"p99_us":4.0}"#,
        );
        assert!(unordered.is_err());

        let incomplete = BaselineMetric::from_json_line(
            r#"{"type":"radiant_perf","scenario":"bad","avg_us":2.5,"p50_us":1.0}"#,
        );
        assert!(incomplete.is_err());
    }
}
