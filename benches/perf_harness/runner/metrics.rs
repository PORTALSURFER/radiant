//! Metric emission for performance harness scenarios.

use std::time::Duration;

use super::{
    OutputFormat,
    baseline::{BaselineMetric, MetricComparison, baseline_metric_json_line, json_escape},
};

const COUNTER_FIELDS: [&str; 27] = [
    "scene_rebuild_count",
    "static_rebuild_count",
    "paint_only_count",
    "surface_refresh_count",
    "relayout_count",
    "dirty_mark_count",
    "overlay_paint_count",
    "overlay_rebuild_count",
    "paint_primitive_count",
    "encoded_paint_primitive_count",
    "scene_append_count",
    "text_cache_hit_count",
    "retained_surface_cache_hit_count",
    "gpu_surface_count",
    "frame_cadence_due_count",
    "frame_cadence_wait_count",
    "widget_callback_allocation_count",
    "text_storage_allocation_count",
    "allocation_sensitive_work_count",
    "gpu_surface_occlusion_primitive_visit_count",
    "gpu_surface_occlusion_index_node_visit_count",
    "gpu_surface_occlusion_candidate_visit_count",
    "application_projection_count",
    "runtime_projection_count",
    "widget_state_sync_count",
    "layout_count",
    "paint_plan_rebuild_count",
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ScenarioCounters {
    values: [Option<u64>; COUNTER_FIELDS.len()],
}

impl ScenarioCounters {
    const SCENE_REBUILD_COUNT: usize = 0;
    const STATIC_REBUILD_COUNT: usize = 1;
    const PAINT_ONLY_COUNT: usize = 2;
    const SURFACE_REFRESH_COUNT: usize = 3;
    const RELAYOUT_COUNT: usize = 4;
    const DIRTY_MARK_COUNT: usize = 5;
    const OVERLAY_PAINT_COUNT: usize = 6;
    const OVERLAY_REBUILD_COUNT: usize = 7;
    const PAINT_PRIMITIVE_COUNT: usize = 8;
    const ENCODED_PAINT_PRIMITIVE_COUNT: usize = 9;
    const SCENE_APPEND_COUNT: usize = 10;
    const TEXT_CACHE_HIT_COUNT: usize = 11;
    const RETAINED_SURFACE_CACHE_HIT_COUNT: usize = 12;
    const GPU_SURFACE_COUNT: usize = 13;
    const FRAME_CADENCE_DUE_COUNT: usize = 14;
    const FRAME_CADENCE_WAIT_COUNT: usize = 15;
    const WIDGET_CALLBACK_ALLOCATION_COUNT: usize = 16;
    const TEXT_STORAGE_ALLOCATION_COUNT: usize = 17;
    const ALLOCATION_SENSITIVE_WORK_COUNT: usize = 18;
    const GPU_SURFACE_OCCLUSION_PRIMITIVE_VISIT_COUNT: usize = 19;
    const GPU_SURFACE_OCCLUSION_INDEX_NODE_VISIT_COUNT: usize = 20;
    const GPU_SURFACE_OCCLUSION_CANDIDATE_VISIT_COUNT: usize = 21;
    const APPLICATION_PROJECTION_COUNT: usize = 22;
    const RUNTIME_PROJECTION_COUNT: usize = 23;
    const WIDGET_STATE_SYNC_COUNT: usize = 24;
    const LAYOUT_COUNT: usize = 25;
    const PAINT_PLAN_REBUILD_COUNT: usize = 26;

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

    pub(crate) fn with_static_rebuild_count(mut self, value: u64) -> Self {
        self.values[Self::STATIC_REBUILD_COUNT] = Some(value);
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

    pub(crate) fn with_encoded_paint_primitive_count(mut self, value: u64) -> Self {
        self.values[Self::ENCODED_PAINT_PRIMITIVE_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_scene_append_count(mut self, value: u64) -> Self {
        self.values[Self::SCENE_APPEND_COUNT] = Some(value);
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

    pub(crate) fn with_widget_callback_allocation_count(mut self, value: u64) -> Self {
        self.values[Self::WIDGET_CALLBACK_ALLOCATION_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_text_storage_allocation_count(mut self, value: u64) -> Self {
        self.values[Self::TEXT_STORAGE_ALLOCATION_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_allocation_sensitive_work_count(mut self, value: u64) -> Self {
        self.values[Self::ALLOCATION_SENSITIVE_WORK_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_gpu_surface_occlusion_primitive_visit_count(mut self, value: u64) -> Self {
        self.values[Self::GPU_SURFACE_OCCLUSION_PRIMITIVE_VISIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_gpu_surface_occlusion_index_node_visit_count(mut self, value: u64) -> Self {
        self.values[Self::GPU_SURFACE_OCCLUSION_INDEX_NODE_VISIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_gpu_surface_occlusion_candidate_visit_count(mut self, value: u64) -> Self {
        self.values[Self::GPU_SURFACE_OCCLUSION_CANDIDATE_VISIT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_application_projection_count(mut self, value: u64) -> Self {
        self.values[Self::APPLICATION_PROJECTION_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_runtime_projection_count(mut self, value: u64) -> Self {
        self.values[Self::RUNTIME_PROJECTION_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_widget_state_sync_count(mut self, value: u64) -> Self {
        self.values[Self::WIDGET_STATE_SYNC_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_layout_count(mut self, value: u64) -> Self {
        self.values[Self::LAYOUT_COUNT] = Some(value);
        self
    }

    pub(crate) fn with_paint_plan_rebuild_count(mut self, value: u64) -> Self {
        self.values[Self::PAINT_PLAN_REBUILD_COUNT] = Some(value);
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ScenarioPercentiles {
    pub(super) p50_us: f64,
    pub(super) p95_us: f64,
    pub(super) p99_us: f64,
}

pub(super) struct MetricRequest<'a> {
    pub(super) name: &'a str,
    pub(super) category: &'a str,
    pub(super) group: &'a str,
    pub(super) iterations: usize,
}

impl ScenarioMetric {
    pub(super) fn print(
        request: MetricRequest<'_>,
        elapsed: Duration,
        counters: ScenarioCounters,
        samples_us: &[f64],
        output_format: OutputFormat,
        baseline: Option<Option<&BaselineMetric>>,
    ) -> Self {
        let total_us = elapsed.as_micros();
        let avg_us = total_us as f64 / request.iterations.max(1) as f64;
        let percentiles = nearest_rank_percentiles(samples_us);
        let comparison = baseline.map(|baseline| MetricComparison::new(avg_us, baseline));
        let baseline_jsonl =
            baseline_metric_json_line(&request, total_us, avg_us, percentiles, counters);
        match output_format {
            OutputFormat::Text => print_text_metric(
                &request,
                total_us,
                avg_us,
                percentiles,
                counters,
                comparison,
            ),
            OutputFormat::JsonLines => print_json_metric(
                &request,
                total_us,
                avg_us,
                percentiles,
                counters,
                comparison,
            ),
        }
        Self {
            comparison,
            baseline_jsonl,
        }
    }
}

fn print_text_metric(
    request: &MetricRequest<'_>,
    total_us: u128,
    avg_us: f64,
    percentiles: ScenarioPercentiles,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) {
    println!(
        "{}",
        text_metric_line(request, total_us, avg_us, percentiles, counters, comparison,)
    );
}

fn text_metric_line(
    request: &MetricRequest<'_>,
    total_us: u128,
    avg_us: f64,
    percentiles: ScenarioPercentiles,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) -> String {
    let name = request.name;
    let category = request.category;
    let group = request.group;
    let iterations = request.iterations;
    let counter_fields = text_counter_fields(counters);
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => format!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} p50_us={:.3} p95_us={:.3} p99_us={:.3}{counter_fields} baseline_avg_us={baseline_avg_us:.3} baseline_ratio={ratio:.3} baseline_status={status}",
            percentiles.p50_us, percentiles.p95_us, percentiles.p99_us,
        ),
        Some(MetricComparison::Missing) => format!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} p50_us={:.3} p95_us={:.3} p99_us={:.3}{counter_fields} baseline_status=missing",
            percentiles.p50_us, percentiles.p95_us, percentiles.p99_us,
        ),
        None => format!(
            "radiant_perf scenario={name} category={category} group={group} iterations={iterations} total_us={total_us} avg_us={avg_us:.3} p50_us={:.3} p95_us={:.3} p99_us={:.3}{counter_fields}",
            percentiles.p50_us, percentiles.p95_us, percentiles.p99_us,
        ),
    }
}

fn print_json_metric(
    request: &MetricRequest<'_>,
    total_us: u128,
    avg_us: f64,
    percentiles: ScenarioPercentiles,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) {
    println!(
        "{}",
        json_metric_line(request, total_us, avg_us, percentiles, counters, comparison,)
    );
}

fn json_metric_line(
    request: &MetricRequest<'_>,
    total_us: u128,
    avg_us: f64,
    percentiles: ScenarioPercentiles,
    counters: ScenarioCounters,
    comparison: Option<MetricComparison>,
) -> String {
    let counter_fields = json_counter_fields(counters);
    match comparison {
        Some(MetricComparison::Matched {
            baseline_avg_us,
            ratio,
            status,
        }) => format!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"p50_us\":{:.3},\"p95_us\":{:.3},\"p99_us\":{:.3}{counter_fields},\"baseline_avg_us\":{baseline_avg_us:.3},\"baseline_ratio\":{ratio:.3},\"baseline_status\":\"{status}\"}}",
            json_escape(request.name),
            json_escape(request.category),
            json_escape(request.group),
            request.iterations,
            total_us,
            avg_us,
            percentiles.p50_us,
            percentiles.p95_us,
            percentiles.p99_us,
        ),
        Some(MetricComparison::Missing) => format!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"p50_us\":{:.3},\"p95_us\":{:.3},\"p99_us\":{:.3}{counter_fields},\"baseline_status\":\"missing\"}}",
            json_escape(request.name),
            json_escape(request.category),
            json_escape(request.group),
            request.iterations,
            total_us,
            avg_us,
            percentiles.p50_us,
            percentiles.p95_us,
            percentiles.p99_us,
        ),
        None => format!(
            "{{\"type\":\"radiant_perf\",\"scenario\":\"{}\",\"category\":\"{}\",\"group\":\"{}\",\"iterations\":{},\"total_us\":{},\"avg_us\":{:.3},\"p50_us\":{:.3},\"p95_us\":{:.3},\"p99_us\":{:.3}{counter_fields}}}",
            json_escape(request.name),
            json_escape(request.category),
            json_escape(request.group),
            request.iterations,
            total_us,
            avg_us,
            percentiles.p50_us,
            percentiles.p95_us,
            percentiles.p99_us,
        ),
    }
}

pub(super) fn nearest_rank_percentiles(samples_us: &[f64]) -> ScenarioPercentiles {
    assert!(
        !samples_us.is_empty(),
        "percentiles require at least one sample"
    );
    assert!(
        samples_us
            .iter()
            .all(|sample| sample.is_finite() && *sample >= 0.0),
        "percentile samples must be finite and non-negative"
    );

    let mut sorted = samples_us.to_vec();
    sorted.sort_by(f64::total_cmp);
    let nearest_rank = |quantile: f64| {
        let rank = (quantile * sorted.len() as f64).ceil() as usize;
        sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
    };
    let percentiles = ScenarioPercentiles {
        p50_us: nearest_rank(0.50),
        p95_us: nearest_rank(0.95),
        p99_us: nearest_rank(0.99),
    };
    assert!(
        percentiles.p50_us.is_finite()
            && percentiles.p95_us.is_finite()
            && percentiles.p99_us.is_finite()
    );
    assert!(percentiles.p50_us <= percentiles.p95_us && percentiles.p95_us <= percentiles.p99_us);
    percentiles
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

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::{
        MetricRequest, ScenarioCounters, ScenarioPercentiles, json_metric_line,
        nearest_rank_percentiles, text_metric_line,
    };
    use serde_json::Value;

    #[test]
    fn nearest_rank_percentiles_sort_and_round_up() {
        assert_eq!(
            nearest_rank_percentiles(&[4.0, 1.0, 3.0, 2.0]),
            ScenarioPercentiles {
                p50_us: 2.0,
                p95_us: 4.0,
                p99_us: 4.0,
            }
        );
    }

    #[test]
    fn metric_serialization_includes_finite_ordered_percentiles() {
        let request = MetricRequest {
            name: "runtime_test",
            category: "runtime_surface",
            group: "standalone_gui",
            iterations: 4,
        };
        let percentiles = nearest_rank_percentiles(&[1.0, 4.0, 2.0, 3.0]);
        let json = json_metric_line(
            &request,
            10,
            2.5,
            percentiles,
            ScenarioCounters::default().with_paint_only_count(1),
            None,
        );
        let value: Value = serde_json::from_str(&json).expect("metric JSON should parse");
        assert_eq!(value["p50_us"], 2.0);
        assert_eq!(value["p95_us"], 4.0);
        assert_eq!(value["p99_us"], 4.0);
        assert_eq!(value["paint_only_count"], 1);

        let text = text_metric_line(
            &request,
            10,
            2.5,
            percentiles,
            ScenarioCounters::default(),
            None,
        );
        assert!(text.contains("p50_us=2.000 p95_us=4.000 p99_us=4.000"));
    }
}
