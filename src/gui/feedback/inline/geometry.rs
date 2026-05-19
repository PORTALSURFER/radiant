use crate::gui::types::{Point, Rect};

use super::{
    model::{InlineIndicatorAnchor, InlineIndicatorLayout, InlineIndicatorMetrics},
    sanitize::{finite_nonnegative, finite_or},
};

/// Return the total width reserved for an inline indicator cluster and text gap.
pub fn inline_indicator_reserved_width(count: usize, metrics: InlineIndicatorMetrics) -> f32 {
    let count = count.min(metrics.max_count).min(8);
    if count == 0 {
        return 0.0;
    }
    let unit_width = finite_nonnegative(metrics.unit_width);
    let unit_gap = finite_nonnegative(metrics.unit_gap);
    (count as f32 * unit_width)
        + ((count.saturating_sub(1)) as f32 * unit_gap)
        + finite_nonnegative(metrics.text_gap)
}

/// Place a compact inline indicator cluster after rendered text.
pub fn inline_indicator_layout(
    anchor: InlineIndicatorAnchor,
    count: usize,
    metrics: InlineIndicatorMetrics,
) -> Option<InlineIndicatorLayout> {
    let count = count.min(metrics.max_count).min(8);
    let content_rect = anchor.content_rect;
    if count == 0 || !content_rect.has_finite_positive_area() {
        return None;
    }
    let unit_height = finite_nonnegative(metrics.unit_height).min(content_rect.height().max(1.0));
    let unit_width = finite_nonnegative(metrics.unit_width).min(content_rect.width().max(1.0));
    if unit_width <= 0.0 || unit_height <= 0.0 {
        return None;
    }
    let unit_gap = finite_nonnegative(metrics.unit_gap);
    let total_width = (count as f32 * unit_width) + ((count.saturating_sub(1)) as f32 * unit_gap);
    let text_origin_x = finite_or(anchor.text_origin_x, content_rect.min.x);
    let text_width = finite_nonnegative(anchor.text_width);
    let text_gap = finite_nonnegative(metrics.text_gap);
    let ideal_start_x = text_origin_x + text_width + text_gap;
    let right_limit_x = finite_or(anchor.right_limit_x, content_rect.max.x)
        .clamp(content_rect.min.x, content_rect.max.x);
    let max_start_x = (right_limit_x - total_width).max(content_rect.min.x);
    let start_x = ideal_start_x.clamp(content_rect.min.x, max_start_x);
    let min_y = content_rect.min.y + ((content_rect.height() - unit_height) * 0.5).floor();
    let max_y = (min_y + unit_height).min(content_rect.max.y);
    let mut rects = [Rect::from_min_max(content_rect.min, content_rect.min); 8];
    for (index, rect) in rects.iter_mut().enumerate().take(count) {
        let min_x = start_x + index as f32 * (unit_width + unit_gap);
        *rect = Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + unit_width).min(content_rect.max.x), max_y),
        );
    }
    Some(InlineIndicatorLayout { rects, count })
}
