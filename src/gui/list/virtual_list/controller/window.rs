use super::VirtualListController;
use crate::gui::list::{VirtualListWindow, VirtualListWindowRequest, resolve_virtual_list_window};

impl VirtualListController {
    /// Resolve and store the current materialized window.
    pub fn resolve(&mut self) -> VirtualListWindow {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: self.total_items,
            viewport_len: self.viewport_len,
            requested_start: self.viewport_start,
            overscan: self.overscan,
            focused_index: self.focused_index,
            previous_start: Some(self.viewport_start),
            guard_band: self.guard_band,
        });
        self.viewport_start = window.viewport_start;
        window
    }
}
