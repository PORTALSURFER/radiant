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
    let visible_rows = bounded_list_visible_rows(item_count, max_visible_rows);
    if visible_rows == 0 {
        return 0.0;
    }
    visible_rows as f32 * finite_nonnegative(row_height) + finite_nonnegative(chrome_height)
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
