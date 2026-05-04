use super::*;

pub(in crate::gui::native_shell::state) fn content_list_row_index_at_point(
    rows: &[CachedContentRow],
    point: Point,
    list_rect: Rect,
) -> Option<usize> {
    if rows.is_empty() || !list_rect.contains(point) {
        return None;
    }
    stacked_row_index_at_point(rows, point)
}

/// Resolve one content-list row index from stacked row geometry in constant time.
pub(in crate::gui::native_shell::state) fn stacked_row_index_at_point(
    rows: &[CachedContentRow],
    point: Point,
) -> Option<usize> {
    let first = rows.first()?;
    if point.x < first.rect.min.x || point.x > first.rect.max.x {
        return None;
    }
    let row_height = first.rect.height().max(0.0);
    let stride = if rows.len() > 1 {
        (rows[1].rect.min.y - first.rect.min.y).max(1.0)
    } else {
        row_height.max(1.0)
    };
    let relative_y = point.y - first.rect.min.y;
    if relative_y < 0.0 {
        return None;
    }
    let index = (relative_y / stride).floor() as usize;
    if index >= rows.len() {
        return None;
    }
    let row_start = first.rect.min.y + (index as f32 * stride);
    let row_end = row_start + row_height;
    if index > 0 {
        let previous_end = row_start - stride + row_height;
        if point.y <= previous_end {
            return Some(index - 1);
        }
    }
    ((point.y >= row_start) && (point.y <= row_end)).then_some(index)
}
