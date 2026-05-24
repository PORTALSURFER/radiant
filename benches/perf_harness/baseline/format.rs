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
