/// Split one delimited inline badge payload into stable per-badge labels.
pub fn inline_badge_labels<'a>(
    text: &'a str,
    delimiter: &'a str,
) -> impl Iterator<Item = &'a str> + 'a {
    text.split(delimiter)
        .map(str::trim)
        .filter(|label| !label.is_empty())
}

/// Materialize inline badge labels once when a cache boundary owns them.
pub fn inline_badge_labels_owned(text: &str, delimiter: &str) -> Vec<String> {
    let mut labels = Vec::new();
    inline_badge_labels_owned_into(text, delimiter, &mut labels);
    labels
}

/// Materialize inline badge labels into caller-owned storage.
///
/// This is the allocation-reusing counterpart to
/// [`inline_badge_labels_owned`] for hosts that repeatedly resolve badge
/// clusters during layout, paint, or cache refreshes.
pub fn inline_badge_labels_owned_into(text: &str, delimiter: &str, labels: &mut Vec<String>) {
    labels.clear();
    labels.extend(inline_badge_labels(text, delimiter).map(str::to_owned));
}
