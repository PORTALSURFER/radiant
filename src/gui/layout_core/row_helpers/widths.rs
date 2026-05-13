/// Return the width of one fixed-width control group.
pub fn fixed_width_group_width(item_width: f32, item_count: usize, gap: f32) -> f32 {
    if item_width <= 0.0 || item_count == 0 {
        return 0.0;
    }
    (item_width * item_count as f32) + (gap.max(0.0) * item_count.saturating_sub(1) as f32)
}

/// Return the total width of fixed-width control groups separated by `group_gap`.
pub fn grouped_fixed_width_row_width(
    item_width: f32,
    group_counts: &[usize],
    gap: f32,
    group_gap: f32,
) -> f32 {
    if item_width <= 0.0 || group_counts.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    let mut visible_groups = 0usize;
    for count in group_counts.iter().copied().filter(|count| *count > 0) {
        total += fixed_width_group_width(item_width, count, gap);
        visible_groups += 1;
    }
    if visible_groups > 1 {
        total += group_gap.max(0.0) * visible_groups.saturating_sub(1) as f32;
    }
    total
}
