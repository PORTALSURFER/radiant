/// Return the suffix of `widths` that fits in `available_width`.
///
/// This preserves the rightmost items for compact action clusters and returns
/// widths in their original order.
pub fn visible_suffix_widths(widths: &[f32], available_width: f32, gap: f32) -> Vec<f32> {
    let mut visible = Vec::new();
    visible_suffix_widths_into(widths, available_width, gap, &mut visible);
    visible
}

/// Write the suffix of `widths` that fits in `available_width` into `visible`.
///
/// The output is cleared before use and retains its allocation for callers that
/// recalculate compact control clusters during repeated layout or paint passes.
pub fn visible_suffix_widths_into(
    widths: &[f32],
    available_width: f32,
    gap: f32,
    visible: &mut Vec<f32>,
) {
    visible.clear();
    if available_width <= 0.0 || widths.is_empty() {
        return;
    }
    let gap = gap.max(0.0);
    let mut used = 0.0;
    let mut first_visible = widths.len();
    for (index, width) in widths.iter().rev().enumerate() {
        let width = width.max(0.0);
        let candidate = used + width + if index > 0 { gap } else { 0.0 };
        if candidate > available_width {
            break;
        }
        first_visible -= 1;
        used = candidate;
    }
    let suffix = &widths[first_visible..];
    if suffix.len() > visible.capacity() {
        visible.reserve(suffix.len());
    }
    visible.extend(suffix.iter().map(|width| width.max(0.0)));
}

/// Resolve a fixed item width that fits `item_count` items after reserved gaps.
pub fn fixed_width_item_extent_for_available_width(
    available_width: f32,
    item_count: usize,
    reserved_gap_width: f32,
    min_item_width: f32,
    max_item_width: f32,
) -> f32 {
    if available_width <= 0.0 || item_count == 0 {
        return 0.0;
    }
    let raw_width = (available_width - reserved_gap_width.max(0.0)) / item_count as f32;
    if raw_width <= 0.0 {
        0.0
    } else {
        let min_item_width = min_item_width.max(0.0);
        let max_item_width = max_item_width.max(min_item_width);
        raw_width.floor().clamp(min_item_width, max_item_width)
    }
}
