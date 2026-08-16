use super::super::metrics::{
    MetricRequest, ScenarioCounters, ScenarioPercentiles, json_counter_fields,
};

pub(in crate::runner) fn json_escape(value: &str) -> String {
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

pub(in crate::runner) fn baseline_metric_json_line(
    request: &MetricRequest<'_>,
    total_us: u128,
    avg_us: f64,
    percentiles: ScenarioPercentiles,
    counters: ScenarioCounters,
) -> String {
    let counter_fields = json_counter_fields(counters);
    format!(
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
    )
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use super::baseline_metric_json_line;
    use crate::runner::metrics::{MetricRequest, ScenarioCounters, ScenarioPercentiles};
    use serde_json::Value;

    #[test]
    fn baseline_jsonl_serializes_percentiles_additively() {
        let request = MetricRequest {
            name: "runtime_test",
            category: "runtime_surface",
            group: "standalone_gui",
            iterations: 4,
        };
        let line = baseline_metric_json_line(
            &request,
            10,
            2.5,
            ScenarioPercentiles {
                p50_us: 2.0,
                p95_us: 4.0,
                p99_us: 4.0,
            },
            ScenarioCounters::default(),
        );
        let value: Value = serde_json::from_str(&line).expect("baseline JSONL should parse");
        assert_eq!(value["avg_us"], 2.5);
        assert_eq!(value["p50_us"], 2.0);
        assert_eq!(value["p95_us"], 4.0);
        assert_eq!(value["p99_us"], 4.0);
    }
}
