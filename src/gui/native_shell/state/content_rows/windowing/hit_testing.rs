use super::*;
use crate::gui::list::{
    MaterializedVirtualListItem, VirtualListItemKey, virtual_list_stacked_item_at_point,
};

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
    let items = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            MaterializedVirtualListItem::new(
                VirtualListItemKey(row.visible_row as u64),
                index,
                row.rect,
            )
        })
        .collect::<Vec<_>>();
    virtual_list_stacked_item_at_point(&items, point)
}
