use super::VirtualListController;
use crate::gui::list::{
    VirtualListWindow, VirtualListWindowChange, VirtualListWindowRequest,
    resolve_virtual_list_window,
};

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

    /// Apply a runtime-originated fixed-row window change and return the
    /// controller's resolved window.
    ///
    /// This is the stateful companion to
    /// `virtual_list_windowed(...).on_window_changed(...)`: the runtime owns the
    /// pixel scroll container, while the host keeps durable item-index viewport
    /// state in this controller. The focus anchor is cleared because
    /// scroll-window changes represent direct runtime viewport movement.
    pub fn apply_window_change(&mut self, change: VirtualListWindowChange) -> VirtualListWindow {
        let window = change.window;
        self.total_items = window.total_items;
        self.viewport_len = window.viewport_len().max(1);
        self.overscan = window.overscan();
        self.viewport_start = window.viewport_start;
        self.focused_index = None;
        self.resolve()
    }

    /// Return whether an index would be visible in the controller's viewport
    /// after applying the provided item and viewport counts.
    ///
    /// Use this before reconfiguring a list after filtering, sorting, or
    /// selection changes when an already-visible focus should not force a
    /// scroll jump.
    pub fn viewport_contains_index(
        &self,
        total_items: usize,
        viewport_len: usize,
        index: usize,
    ) -> bool {
        if total_items == 0 || viewport_len == 0 || index >= total_items {
            return false;
        }
        let viewport_len = viewport_len.min(total_items);
        let max_start = total_items.saturating_sub(viewport_len);
        let viewport_start = self.viewport_start.min(max_start);
        let window = VirtualListWindow {
            total_items,
            viewport_start,
            viewport_end: viewport_start.saturating_add(viewport_len),
            window_start: viewport_start,
            window_end: viewport_start.saturating_add(viewport_len),
        };
        window.viewport_contains(index)
    }
}
