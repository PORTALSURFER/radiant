use crate::gui::list::{
    VirtualListWindow, VirtualListWindowChange, VirtualListWindowRequest,
    resolve_virtual_list_window, virtual_list_view_start_for_scroll_offset,
};
use crate::runtime::ScrollUpdate;

pub(super) fn resolve_virtual_list_window_change(
    offset_y: f32,
    row_height: f32,
    viewport_height: f32,
    current: VirtualListWindow,
    overscan_px: f32,
) -> VirtualListWindowChange {
    let row_height = row_height.max(1.0);
    let requested_start =
        virtual_list_view_start_for_scroll_offset(offset_y, row_height, current.total_items);
    let viewport_len = (viewport_height.max(0.0) / row_height).ceil().max(1.0) as usize;
    let overscan = (overscan_px.max(0.0) / row_height).ceil() as usize;
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: current.total_items,
        viewport_len,
        requested_start,
        overscan,
        focused_index: None,
        previous_start: None,
        guard_band: 0,
    });
    VirtualListWindowChange {
        offset_y,
        row_height,
        window,
    }
}

/// Return whether a runtime scroll update needs the host to materialize a new
/// row window.
///
/// A fixed-row scroll container can continue moving while its visible viewport
/// remains inside the rows already projected by the host. Deferring host
/// updates for that interval avoids rebuilding the retained row widgets for
/// every wheel or touchpad increment. A changed viewport height must still be
/// reported so the host can keep its logical viewport contract in sync.
pub(crate) fn virtual_list_window_needs_materialization(
    current: VirtualListWindow,
    next: VirtualListWindow,
    offset_y: f32,
    row_height: f32,
    viewport_height: f32,
) -> bool {
    let row_height = row_height.max(1.0);
    let visible_end = ((offset_y.max(0.0) + viewport_height.max(0.0)) / row_height)
        .ceil()
        .max(0.0) as usize;
    let visible_end = visible_end.min(next.total_items);
    current.total_items != next.total_items
        || current.viewport_len() != next.viewport_len()
        || next.viewport_start < current.window_start
        || visible_end > current.window_end
}

/// Resolve a fixed-row virtual-list window change from a runtime scroll update.
pub fn virtual_list_window_change_for_scroll(
    update: ScrollUpdate,
    row_height: f32,
    current: VirtualListWindow,
    overscan_rows: usize,
) -> VirtualListWindowChange {
    resolve_virtual_list_window_change(
        update.offset.y,
        row_height,
        update.viewport.y,
        current,
        row_height.max(0.0) * overscan_rows as f32,
    )
}
