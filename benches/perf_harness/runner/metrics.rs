//! Metric emission for performance harness scenarios.

use std::time::Duration;

use super::{
    OutputFormat,
    baseline::{BaselineMetric, MetricComparison, baseline_metric_json_line, json_escape},
};

const COUNTER_FIELDS: [&str; 14] = [
    "scene_rebuild_count",
    "paint_only_count",
    "surface_refresh_count",
    "relayout_count",
    "dirty_mark_count",
    "overlay_paint_count",
    "overlay_rebuild_count",
    "paint_primitive_count",
    "text_cache_hit_count",
    "retained_surface_cache_hit_count",
    "gpu_surface_count",
    "frame_cadence_due_count",
    "frame_cadence_wait_count",
    "allocation_sensitive_work_count",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScenarioCounters {
    values: [Option<u64>; COUNTER_FIELDS.len()],
}

impl ScenarioCounters {
    const SCENE_REBUILD_COUNT: usize = 0;
    const PAINT_ONLY_COUNT: usize = 1;
    const SURFACE_REFRESH_COUNT: usize = 2;
    const RELAYOUT_COUNT: usize = 3;
    const DIRTY_MARK_COUNT: usize = 4;
    const OVERLAY_PAINT_COUNT: usize = 5;
    const OVERLAY_REBUILD_COUNT: usize = 6;
    const PAINT_PRIMITIVE_COUNT: usize = 7;
    const TEXT_CACHE_HIT_COUNT: usize = 8;
    const RETAINED_SURFACE_CACHE_HIT_COUNT: usize = 9;
    const GPU_SURFACE_COUNT: usize = 10;
    const FRAME_CADENCE_DUE_COUNT: usize = 11;
    const FRAME_CADENCE_WAIT_COUNT: usize = 12;
    const ALLOCATION_SENSITIVE_WORK_COUNT: usize = 13;

    pub(crate) fn add(&mut self, other: Self) {
        for (index, value) in other.values.into_iter().enumerate() {
            if let Some(value) = value {
                let current = self.values[index].unwrap_or(0);
                self.values[index] = Some(current.saturating_add(value));
            }
        }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.values.iter().all(Option::is_none)
    }

    pub(crate) fn iter(self) -> impl Iterator<Item = (&'static str, u64)> {
        COUNTER_FIELDS
            .into_iter()
            .zip(self.values)
            .filter_map(|(name, value)| value.map(|value| (name, value)))
    }

    pub(crate) fn with_scene_rebuild_count(mut self, value: u64) -> Self {
        self.values[Self::SCENE_REBUILD_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_paint_only_count(mut self, value: u64) -> Self {
        self.values[Self::PAINT_ONLY_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_surface_refresh_count(mut self, value: u64) -> Self {
        self.values[Self::SURFACE_REFRESH_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_relayout_count(mut self, value: u64) -> Self {
        self.values[Self::RELAYOUT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_dirty_mark_count(mut self, value: u64) -> Self {
        self.values[Self::DIRTY_MARK_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_overlay_paint_count(mut self, value: u64) -> Self {
        self.values[Self::OVERLAY_PAINT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_overlay_rebuild_count(mut self, value: u64) -> Self {
        self.values[Self::OVERLAY_REBUILD_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_paint_primitive_count(mut self, value: u64) -> Self {
        self.values[Self::PAINT_PRIMITIVE_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_text_cache_hit_count(mut self, value: u64) -> Self {
        self.values[Self::TEXT_CACHE_HIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_retained_surface_cache_hit_count(mut self, value: u64) -> Self {
        self.values[Self::RETAINED_SURFACE_CACHE_HIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_gpu_surface_count(mut self, value: u64) -> Self {
        self.values[Self::GPU_SURFACE_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_frame_cadence_due_count(mut self, value: u64) -> Self {
        self.values[Self::FRAME_CADENCE_DUE_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_frame_cadence_wait_count(mut self, value: u64) -> Self {
        self.values[Self::FRAME_CADENCE_WAIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_allocation_sensitive_work_count(mut self, value: u64) -> Self {
        self.values[Self::ALLOCATION_SENSITIVE_WORK_COUNT] = Some(value);
        self
    }
}

impl From<()> for ScenarioCounters {
    fn from(_: ()) -> Self {
        Self::default()
    }
}

pub(super) struct ScenarioMetric {
    pub(super) comparison: Option<MetricComparison>,
    pub(super) baseline_jsonl: String,
}

impl ScenarioMetric {
    pub(super) fn print(
        name: &str,
        category: &str,
        group: &str,
        iterations: usize,
        elapsed: Duration,
        counters: ScenarioCounters,
        output_format: OutputFormat,
        baseline: Option<Option<&BaselineMetric>>,
    ) -> Self {
        let total_us = elapsed.as_micros();
        let avg_us = total_us as f64 / iterations.max(1) as f64;
        let comparison = baseline.map(|baseline| MetricComparison::new(avg_us, baseline));
        let baseline_jsonl = baseline_metric_json_line(
            name, category, group, iterations, total_us, avg_us, counters,
        );
        match output_format {
            OutputFormat::Text => print_text_metric(
                name, category, group, iterations, total_us, avg_us, counters, comparison,
            ),
            OutputFormat::JsonLines => print_json_metric(
                name, category, group, iterations, total_us, avg_us, counters, comparison,
            ),
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
    group: &str,
    iterations: usize,
    total_us: u128,
    avg_us: f64,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) {
    let counter_fields = text_counter_fields(counters);
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => println!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}{counter_fields} baseline_avg_us={baseline_avg_us:.3} baseline_ratio={ratio:.3} baseline_status={status}"
        ),
        Some(MetricComparison::Missing) => println!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}{counter_fields} baseline_status=missing"
        ),
        None => println!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3}{counter_fields}"
        ),
    }
}

fn print_json_metric(
    name: &str,
    category: &str,
    group: &str,
    iterations: usize,
    total_us: u128,
    avg_us: f64,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) {
    let counter_fields = json_counter_fields(counters);
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}{counter_fields},\"baseline_avg_us\":{baseline_avg_us:.3},\"baseline_ratio\":{ratio:.3},\"baseline_status\":\"{status}\"}}",
            json_escape(name),
            json_escape(category),
            json_escape(group),
            iterations,
            total_us,
            avg_us,
        ),
        Some(MetricComparison::Missing) => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}{counter_fields},\"baseline_status\":\"missing\"}}",
            json_escape(name),
            json_escape(category),
            json_escape(group),
            iterations,
            total_us,
            avg_us,
        ),
        None => println!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3}{counter_fields}}}",
            json_escape(name),
            json_escape(category),
            json_escape(group),
            iterations,
            total_us,
            avg_us
        ),
    }
}

fn text_counter_fields(counters: ScenarioCounters) -> String {
    if counters.is_empty() {
        return String::new();
    }
    counters
        .iter()
        .map(|(name, value)| format!(" {name}={value}"))
        .collect()
}

pub(crate) fn json_counter_fields(counters: ScenarioCounters) -> String {
    if counters.is_empty() {
        return String::new();
    }
    counters
        .iter()
        .map(|(name, value)| format!(",\"{name}\":{value}"))
        .collect()
}
