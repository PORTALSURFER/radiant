use crate::gui::{
    badge::inline::{InlineBadgeMetrics, inline_badge_labels_owned_into},
    types::{Point, Rect},
};

/// Approximate the rendered width of one inline badge label.
pub fn inline_badge_text_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    ((text.chars().count() as f32) * (metrics.font_size * 0.56).max(1.0)).ceil()
}

/// Return the filled badge width needed for one inline badge label.
pub fn inline_badge_width(text: &str, metrics: InlineBadgeMetrics) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    inline_badge_text_width(text, metrics) + (metrics.padding_x * 2.0)
}

/// Return a badge width clamped to a caller-defined logical-width range.
pub fn inline_badge_width_in_range(
    text: &str,
    metrics: InlineBadgeMetrics,
    min_width: f32,
    max_width: f32,
) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let min_width = finite_nonnegative_width(min_width);
    let max_width = finite_nonnegative_width(max_width).max(min_width);
    inline_badge_width(text, metrics).clamp(min_width, max_width)
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

fn finite_nonnegative_width(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
