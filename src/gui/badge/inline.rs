//! Inline badge text measurement and row geometry helpers.

use crate::gui::types::{Point, Rect};

/// Layout metrics for a compact inline badge cluster.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineBadgeMetrics {
    /// Font size used for label measurement and vertical text placement.
    pub font_size: f32,
    /// Horizontal inset inside each badge.
    pub padding_x: f32,
    /// Vertical inset inside each badge.
    pub padding_y: f32,
    /// Horizontal gap between adjacent badges.
    pub badge_gap: f32,
    /// Gap between the host item label and the badge cluster.
    pub cluster_gap: f32,
    /// Minimum desired badge height before clamping to the available row height.
    pub min_height: f32,
}

impl InlineBadgeMetrics {
    /// Construct metrics from already-resolved geometry tokens.
    pub fn new(
        font_size: f32,
        padding_x: f32,
        padding_y: f32,
        badge_gap: f32,
        cluster_gap: f32,
        min_height: f32,
    ) -> Self {
        Self {
            font_size,
            padding_x,
            padding_y,
            badge_gap,
            cluster_gap,
            min_height,
        }
    }
}

/// Approximate the rendered width of one inline badge label.
pub fn inline_badge_text_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (metrics.font_size * 0.56).max(1.0)).ceil()
}

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

/// Return the filled badge width needed for one inline badge label.
pub fn inline_badge_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    inline_badge_text_width(text, metrics) + (metrics.padding_x * 2.0)
}

/// Return reserved width for a pre-split inline badge cluster.
pub fn inline_badge_cluster_reserved_width(labels: &[String], metrics: InlineBadgeMetrics) -> f32 {
    if labels.is_empty() {
        return 0.0;
    }
    let badges_width = labels
        .iter()
        .map(|label| inline_badge_width(label, metrics))
        .sum::<f32>();
    let badge_gap_count = labels.len().saturating_sub(1) as f32;
    badges_width + (badge_gap_count * metrics.badge_gap) + metrics.cluster_gap
}

/// Return the desired badge height for one item-label row.
pub fn inline_badge_height(item_label: Rect, metrics: InlineBadgeMetrics) -> f32 {
    let available_height = item_label.height().max(0.0);
    if available_height <= 0.0 {
        return 0.0;
    }
    let desired_height = (metrics.font_size + (metrics.padding_y * 2.0)).round();
    let min_height = metrics.min_height.min(available_height);
    desired_height.clamp(min_height, available_height)
}

/// Compute badge rects for pre-split inline badge labels.
pub fn inline_badge_rects_for_labels(
    item_label: Rect,
    labels: &[String],
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    inline_badge_rects_for_labels_into(
        item_label,
        labels,
        trailing_reserved_width,
        metrics,
        &mut rects,
    );
    rects
}

/// Compute badge rects for pre-split labels into caller-owned storage.
pub fn inline_badge_rects_for_labels_into(
    item_label: Rect,
    labels: &[String],
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
    rects: &mut Vec<Rect>,
) {
    rects.clear();
    if labels.is_empty() || item_label.width() <= 0.0 || item_label.height() <= 0.0 {
        return;
    }
    let total_width = labels
        .iter()
        .map(|label| inline_badge_width(label, metrics))
        .sum::<f32>()
        + (labels.len().saturating_sub(1) as f32 * metrics.badge_gap);
    let right_edge = (item_label.max.x - trailing_reserved_width).max(item_label.min.x);
    let start_x = (right_edge - total_width).max(item_label.min.x);
    let badge_height = inline_badge_height(item_label, metrics);
    if badge_height <= 0.0 || right_edge <= start_x {
        return;
    }
    let min_y = item_label.min.y + ((item_label.height() - badge_height) * 0.5).floor();
    let max_y = (min_y + badge_height).min(item_label.max.y);
    if max_y <= min_y {
        return;
    }
    let mut x = start_x;
    if labels.len() > rects.capacity() {
        rects.reserve(labels.len());
    }
    rects.extend(labels.iter().map(|label| {
        let width = inline_badge_width(label, metrics);
        let rect = Rect::from_min_max(
            Point::new(x, min_y),
            Point::new((x + width).min(right_edge), max_y),
        );
        x = (rect.max.x + metrics.badge_gap).min(right_edge);
        rect
    }));
}

/// Compute badge rects for one delimited inline badge cluster.
pub fn inline_badge_rects(
    item_label: Rect,
    text: &str,
    delimiter: &str,
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
) -> Vec<Rect> {
    let mut rects = Vec::new();
    let mut labels = Vec::new();
    inline_badge_rects_into(
        item_label,
        text,
        delimiter,
        trailing_reserved_width,
        metrics,
        &mut labels,
        &mut rects,
    );
    rects
}

/// Compute badge rects for one delimited inline badge cluster into
/// caller-owned label and rect buffers.
pub fn inline_badge_rects_into(
    item_label: Rect,
    text: &str,
    delimiter: &str,
    trailing_reserved_width: f32,
    metrics: InlineBadgeMetrics,
    labels: &mut Vec<String>,
    rects: &mut Vec<Rect>,
) {
    rects.clear();
    labels.clear();
    if text.is_empty() || item_label.width() <= 0.0 || item_label.height() <= 0.0 {
        return;
    }
    inline_badge_labels_owned_into(text, delimiter, labels);
    inline_badge_rects_for_labels_into(item_label, labels, trailing_reserved_width, metrics, rects);
}

/// Return the inset text origin for one inline badge.
pub fn inline_badge_text_origin(badge_rect: Rect, metrics: InlineBadgeMetrics) -> Point {
    Point::new(
        badge_rect.min.x + metrics.padding_x,
        badge_rect.min.y + ((badge_rect.height() - metrics.font_size) * 0.5).floor(),
    )
}
