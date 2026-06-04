/// Return the number of list rows visible inside a bounded list surface.
pub fn bounded_list_visible_rows(item_count: usize, max_visible_rows: usize) -> usize {
    item_count.min(max_visible_rows)
}

/// Return the height for a bounded fixed-row list surface.
///
/// This is useful for autocomplete popups, command palettes, inspector menus,
/// and other transient list surfaces that should show no chrome when empty and
/// cap their visible rows before becoming scrollable.
pub fn bounded_list_height(
    item_count: usize,
    max_visible_rows: usize,
    row_height: f32,
    chrome_height: f32,
) -> f32 {
    bounded_list_height_with_gap(item_count, max_visible_rows, row_height, 0.0, chrome_height)
}

/// Return the content height for fixed-height rows with a gap between rows.
///
/// This is useful for non-virtualized row stacks whose parent owns scrolling or
/// panel chrome but still wants the same empty-list and finite-metric behavior
/// as Radiant's bounded list helpers.
pub fn fixed_row_stack_height(item_count: usize, row_height: f32, row_gap: f32) -> f32 {
    if item_count == 0 {
        return 0.0;
    }
    item_count as f32 * finite_nonnegative(row_height)
        + item_count.saturating_sub(1) as f32 * finite_nonnegative(row_gap)
}

/// Return the height for a bounded fixed-row list surface with inter-row gaps.
///
/// This extends [`bounded_list_height`] for compact menus, panels, and option
/// lists where rows are visually separated but should still hide empty chrome
/// and cap visible rows before scrolling.
pub fn bounded_list_height_with_gap(
    item_count: usize,
    max_visible_rows: usize,
    row_height: f32,
    row_gap: f32,
    chrome_height: f32,
) -> f32 {
    let visible_rows = bounded_list_visible_rows(item_count, max_visible_rows);
    if visible_rows == 0 {
        return 0.0;
    }
    fixed_row_stack_height(visible_rows, row_height, row_gap) + finite_nonnegative(chrome_height)
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
