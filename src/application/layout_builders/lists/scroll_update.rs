use crate::gui::list::{
    VirtualListWindow, VirtualListWindowChange, VirtualListWindowRequest,
    resolve_virtual_list_window, virtual_list_view_start_for_scroll_offset,
};
use crate::runtime::ScrollUpdate;

pub(super) fn resolve_virtual_list_window_change(
    offset_y: f32,
    row_height: f32,
    current: VirtualListWindow,
    overscan_px: f32,
) -> VirtualListWindowChange {
    let row_height = row_height.max(1.0);
    let requested_start =
        virtual_list_view_start_for_scroll_offset(offset_y, row_height, current.total_items);
    let overscan = (overscan_px.max(0.0) / row_height).ceil() as usize;
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: current.total_items,
        viewport_len: current.viewport_len(),
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
        current,
        row_height.max(0.0) * overscan_rows as f32,
    )
}
