use super::VirtualListController;
use crate::gui::list::{
    VirtualListScrollbar, VirtualListScrollbarRequest, VirtualListWindow,
    resolve_virtual_list_scrollbar, virtual_list_scrollbar_view_start_for_pointer,
};
use crate::gui::types::Rect;

impl VirtualListController {
    /// Resolve vertical scrollbar geometry for the current viewport.
    pub fn scrollbar(
        &mut self,
        track: Rect,
        min_thumb_extent: f32,
    ) -> Option<VirtualListScrollbar> {
        let window = self.resolve();
        resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
            track,
            total_items: window.total_items,
            viewport_len: window.viewport_len(),
            viewport_start: window.viewport_start,
            min_thumb_extent,
        })
    }

    /// Update the viewport from a dragged scrollbar thumb.
    pub fn drag_scrollbar(
        &mut self,
        scrollbar: VirtualListScrollbar,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<VirtualListWindow> {
        let window = self.resolve();
        let next = virtual_list_scrollbar_view_start_for_pointer(
            scrollbar,
            window.viewport_len(),
            window.total_items,
            pointer_y,
            thumb_pointer_offset_y,
        )?;
        self.focused_index = None;
        self.viewport_start = next;
        Some(self.resolve())
    }
}
