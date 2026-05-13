//! Linear virtual metrics validity checks.

use crate::gui::layout_core::engine::cache::LinearVirtualMetrics;

/// Return true when virtualized metrics are safe to consume.
pub(in crate::gui::layout_core::engine::layout) fn metrics_is_valid(
    metrics: &LinearVirtualMetrics,
    expected_len: usize,
) -> bool {
    if metrics.len() != expected_len {
        return false;
    }
    if metrics.uniform.is_none()
        && (metrics.spans.len() != expected_len || metrics.main_sizes.len() != expected_len)
    {
        return false;
    }
    if !metrics.total_main.is_finite()
        || !metrics.leading_offset.is_finite()
        || !metrics.distributed_spacing.is_finite()
    {
        return false;
    }
    if let Some(uniform) = metrics.uniform {
        return uniform.main_size.is_finite()
            && uniform.main_size >= 0.0
            && uniform.step.is_finite()
            && uniform.step >= 0.0;
    }
    metrics
        .spans
        .iter()
        .all(|span| span.start.is_finite() && span.end.is_finite() && span.end >= span.start)
}
