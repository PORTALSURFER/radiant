pub(super) fn parse_number_list(raw: &str) -> Option<Vec<f64>> {
    let normalized = raw.replace(',', " ");
    normalized
        .split_whitespace()
        .map(parse_number)
        .collect::<Option<Vec<_>>>()
}

pub(super) fn parse_number(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok()
}
