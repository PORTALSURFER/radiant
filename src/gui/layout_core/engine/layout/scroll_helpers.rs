//! Shared helper logic for scroll virtualization.

use super::super::{LayoutContext, LinearVirtualMetrics};
use crate::gui::types::{Point, Rect, Vector2};

/// Compute a virtualized child window over cached span metrics.
pub(super) fn compute_virtual_window(
    metrics: &LinearVirtualMetrics,
    viewport_start: f32,
    viewport_size: f32,
    overscan_px: f32,
) -> (f32, f32, usize, usize, bool) {
    if metrics.spans.is_empty() {
        return (0.0, 0.0, 0, 0, false);
    }
    let max_main = metrics.total_main.max(0.0);
    let mut clamped = false;
    let safe_viewport_start = if viewport_start.is_finite() {
        viewport_start.clamp(0.0, max_main)
    } else {
        clamped = true;
        0.0
    };
    let safe_viewport_size = if viewport_size.is_finite() {
        viewport_size.max(0.0)
    } else {
        clamped = true;
        0.0
    };
    let start = (safe_viewport_start - overscan_px).clamp(0.0, max_main);
    let end = (safe_viewport_start + safe_viewport_size + overscan_px).clamp(0.0, max_main);
    let first = lower_bound_end(metrics, start);
    let mut last_exclusive = lower_bound_start(metrics, end);
    let mut first_index = first.min(metrics.spans.len());
    if first_index >= last_exclusive && !metrics.spans.is_empty() {
        clamped = true;
        first_index = first_index.min(metrics.spans.len() - 1);
        last_exclusive = (first_index + 1).min(metrics.spans.len());
    }
    (start, end.max(start), first_index, last_exclusive, clamped)
}

/// Compute the pre-first-item cursor position for virtualized linear layout.
pub(super) fn cursor_before_first(
    before_margin: f32,
    first: usize,
    metrics: &LinearVirtualMetrics,
) -> f32 {
    if first >= metrics.spans.len() {
        return 0.0;
    }
    (metrics.spans[first].start - before_margin).max(0.0)
}

/// Record debug primitives for virtualization window and culled regions.
pub(super) fn record_window_debug(
    node_id: u64,
    rect: Rect,
    horizontal: bool,
    window_start: f32,
    window_end: f32,
    context: &mut LayoutContext,
) {
    let window_rect = if horizontal {
        Rect::from_min_size(
            Point::new(rect.min.x + window_start, rect.min.y),
            Vector2::new((window_end - window_start).max(0.0), rect.height()),
        )
    } else {
        Rect::from_min_size(
            Point::new(rect.min.x, rect.min.y + window_start),
            Vector2::new(rect.width(), (window_end - window_start).max(0.0)),
        )
    };
    context.record_virtual_window_bounds(node_id, window_rect);

    if window_start > 0.0 {
        let before_rect = if horizontal {
            Rect::from_min_size(
                rect.min,
                Vector2::new(window_start.min(rect.width()), rect.height()),
            )
        } else {
            Rect::from_min_size(
                rect.min,
                Vector2::new(rect.width(), window_start.min(rect.height())),
            )
        };
        context.record_culled_region(node_id, before_rect);
    }
    let after_start = if horizontal {
        rect.min.x + window_end
    } else {
        rect.min.y + window_end
    };
    if (horizontal && after_start < rect.max.x) || (!horizontal && after_start < rect.max.y) {
        let after_rect = if horizontal {
            Rect::from_min_size(
                Point::new(after_start, rect.min.y),
                Vector2::new((rect.max.x - after_start).max(0.0), rect.height()),
            )
        } else {
            Rect::from_min_size(
                Point::new(rect.min.x, after_start),
                Vector2::new(rect.width(), (rect.max.y - after_start).max(0.0)),
            )
        };
        context.record_culled_region(node_id, after_rect);
    }
}

/// Clamp invalid overscan values to a safe default.
pub(super) fn sanitize_overscan(overscan_px: f32) -> (f32, bool) {
    if !overscan_px.is_finite() || overscan_px < 0.0 {
        return (0.0, true);
    }
    (overscan_px, false)
}

/// Clamp invalid scroll offsets into legal bounds.
pub(super) fn clamp_scroll_offset(requested: Vector2, max_x: f32, max_y: f32) -> (bool, Vector2) {
    let mut req_x = requested.x;
    let mut req_y = requested.y;
    let mut invalid = false;
    if !req_x.is_finite() {
        req_x = 0.0;
        invalid = true;
    }
    if !req_y.is_finite() {
        req_y = 0.0;
        invalid = true;
    }
    let clamped_x = req_x.clamp(0.0, max_x);
    let clamped_y = req_y.clamp(0.0, max_y);
    (
        invalid
            || (clamped_x - req_x).abs() > f32::EPSILON
            || (clamped_y - req_y).abs() > f32::EPSILON,
        Vector2::new(clamped_x, clamped_y),
    )
}

fn lower_bound_end(metrics: &LinearVirtualMetrics, value: f32) -> usize {
    let mut low = 0usize;
    let mut high = metrics.spans.len();
    while low < high {
        let mid = (low + high) / 2;
        if metrics.spans[mid].end <= value {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

fn lower_bound_start(metrics: &LinearVirtualMetrics, value: f32) -> usize {
    let mut low = 0usize;
    let mut high = metrics.spans.len();
    while low < high {
        let mid = (low + high) / 2;
        if metrics.spans[mid].start < value {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}
