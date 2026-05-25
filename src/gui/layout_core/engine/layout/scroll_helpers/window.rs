use super::super::super::cache::LinearVirtualMetrics;

pub(in crate::gui::layout_core::engine::layout) struct ComputedVirtualWindow {
    pub(in crate::gui::layout_core::engine::layout) start: f32,
    pub(in crate::gui::layout_core::engine::layout) end: f32,
    pub(in crate::gui::layout_core::engine::layout) first: usize,
    pub(in crate::gui::layout_core::engine::layout) last_exclusive: usize,
    pub(in crate::gui::layout_core::engine::layout) clamped: bool,
}

impl ComputedVirtualWindow {
    fn empty() -> Self {
        Self {
            start: 0.0,
            end: 0.0,
            first: 0,
            last_exclusive: 0,
            clamped: false,
        }
    }
}

/// Compute a virtualized child window over cached span metrics.
pub(in crate::gui::layout_core::engine::layout) fn compute_virtual_window(
    metrics: &LinearVirtualMetrics,
    viewport_start: f32,
    viewport_size: f32,
    overscan_px: f32,
) -> ComputedVirtualWindow {
    if metrics.is_empty() {
        return ComputedVirtualWindow::empty();
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
    let mut first_index = first.min(metrics.len());
    if first_index >= last_exclusive && !metrics.is_empty() {
        clamped = true;
        first_index = first_index.min(metrics.len() - 1);
        last_exclusive = (first_index + 1).min(metrics.len());
    }
    ComputedVirtualWindow {
        start,
        end: end.max(start),
        first: first_index,
        last_exclusive,
        clamped,
    }
}

fn lower_bound_end(metrics: &LinearVirtualMetrics, value: f32) -> usize {
    if let Some(uniform) = metrics.uniform {
        if uniform.step <= f32::EPSILON {
            return 0;
        }
        let covered = (value - metrics.leading_offset - uniform.main_size) / uniform.step;
        if covered < 0.0 {
            return 0;
        }
        return (covered.floor() as usize + 1).min(uniform.count);
    }
    let mut low = 0usize;
    let mut high = metrics.len();
    while low < high {
        let mid = (low + high) / 2;
        if metrics.span(mid).is_some_and(|span| span.end <= value) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

fn lower_bound_start(metrics: &LinearVirtualMetrics, value: f32) -> usize {
    if let Some(uniform) = metrics.uniform {
        if uniform.step <= f32::EPSILON {
            return 0;
        }
        let covered = (value - metrics.leading_offset) / uniform.step;
        if covered <= 0.0 {
            return 0;
        }
        return covered.ceil().min(uniform.count as f32) as usize;
    }
    let mut low = 0usize;
    let mut high = metrics.len();
    while low < high {
        let mid = (low + high) / 2;
        if metrics.span(mid).is_some_and(|span| span.start < value) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}
