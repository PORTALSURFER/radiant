use super::VirtualListController;
use crate::gui::list::{
    VirtualListWindow, virtual_list_scroll_delta_from_units,
    virtual_list_view_start_after_scroll_delta, virtual_list_view_start_for_scroll_offset,
};

impl VirtualListController {
    /// Request an absolute viewport start and clear focus anchoring.
    pub fn set_viewport_start(&mut self, viewport_start: usize) -> VirtualListWindow {
        self.focused_index = None;
        self.viewport_start = viewport_start;
        self.resolve()
    }

    /// Update the viewport from a logical scroll offset and clear focus anchoring.
    ///
    /// Use this when a native scroll container reports a pixel offset while the
    /// application keeps item-index based virtual-list state.
    pub fn set_scroll_offset(&mut self, offset: f32, row_extent: f32) -> VirtualListWindow {
        self.focused_index = None;
        self.viewport_start =
            virtual_list_view_start_for_scroll_offset(offset, row_extent, self.total_items);
        if self.viewport_len == 0 {
            return VirtualListWindow {
                total_items: self.total_items,
                ..VirtualListWindow::default()
            };
        }
        self.resolve()
    }

    /// Update the item count and viewport from a logical scroll offset.
    ///
    /// Use this when a native scroll container reports a pixel offset while the
    /// host application also owns filters, searches, or selections that can
    /// change the total logical item count between scroll events.
    pub fn set_scroll_offset_for_items(
        &mut self,
        total_items: usize,
        offset: f32,
        row_extent: f32,
    ) -> VirtualListWindow {
        self.set_total_items(total_items);
        self.set_scroll_offset(offset, row_extent)
    }

    /// Scroll the viewport by signed logical rows.
    pub fn scroll_rows(&mut self, rows: isize) -> Option<VirtualListWindow> {
        let next = virtual_list_view_start_after_scroll_delta(
            self.viewport_start,
            self.total_items,
            self.viewport_len,
            rows,
        )?;
        self.focused_index = None;
        self.viewport_start = next;
        Some(self.resolve())
    }

    /// Scroll the viewport by normalized row units.
    pub fn scroll_units(&mut self, units: f32) -> Option<VirtualListWindow> {
        let rows = virtual_list_scroll_delta_from_units(units)?;
        self.scroll_rows(rows as isize)
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualListController;

    #[test]
    fn set_scroll_offset_clamps_to_current_items() {
        let mut controller = VirtualListController::with_items(10, 4);

        let window = controller.set_scroll_offset(99.0 * 22.0, 22.0);

        assert_eq!(window.viewport_start, 6);
        assert_eq!(controller.viewport_start(), 6);
        assert_eq!(controller.focused_index(), None);
    }

    #[test]
    fn set_scroll_offset_preserves_start_before_viewport_is_known() {
        let mut controller = VirtualListController::new();
        controller.set_total_items(24);

        let window = controller.set_scroll_offset(23.0 * 22.0, 22.0);

        assert_eq!(window.total_items, 24);
        assert_eq!(window.viewport_len(), 0);
        assert_eq!(controller.viewport_start(), 23);
    }

    #[test]
    fn set_scroll_offset_for_items_updates_count_before_clamping_scroll() {
        let mut controller = VirtualListController::with_items(100, 10);
        controller.focus(70);

        let window = controller.set_scroll_offset_for_items(18, 99.0 * 22.0, 22.0);

        assert_eq!(window.total_items, 18);
        assert_eq!(window.viewport_start, 8);
        assert_eq!(controller.total_items(), 18);
        assert_eq!(controller.viewport_start(), 8);
        assert_eq!(controller.focused_index(), None);
    }
}
