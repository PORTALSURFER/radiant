use super::VirtualListController;
use crate::gui::list::{
    VirtualListFocusTarget, VirtualListFollowState, VirtualListProjection, VirtualListSliceFocus,
    VirtualListWindow,
};

impl VirtualListController {
    /// Configure the stable geometry inputs and resolve around optional focus.
    ///
    /// Use this during a projection pass when host-owned selection should keep
    /// a virtualized list scrolled near the selected item.
    pub fn configure_and_focus_optional(
        &mut self,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        focused_index: Option<usize>,
    ) -> VirtualListWindow {
        self.configure_projection_and_focus_optional(
            VirtualListProjection::new(total_items, viewport_len, overscan, guard_band),
            focused_index,
        )
    }

    /// Configure named projection inputs and resolve around optional focus.
    pub fn configure_projection_and_focus_optional(
        &mut self,
        projection: VirtualListProjection,
        focused_index: Option<usize>,
    ) -> VirtualListWindow {
        self.configure_projection(projection);
        self.focus_optional(focused_index)
    }

    /// Configure stable geometry inputs and follow optional focus only when an
    /// app-owned focus key changes.
    ///
    /// This preserves manual scroll as authoritative while the same item remains
    /// selected, but still scrolls the newly selected item into view when host
    /// selection moves to another key.
    pub fn configure_and_focus_changed_optional<Key: PartialEq>(
        &mut self,
        follow_state: &mut VirtualListFollowState<Key>,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        focus: VirtualListFocusTarget<Key>,
    ) -> VirtualListWindow {
        self.configure_projection_and_focus_changed_optional(
            follow_state,
            VirtualListProjection::new(total_items, viewport_len, overscan, guard_band),
            focus,
        )
    }

    /// Configure named projection inputs and follow optional focus only when an
    /// app-owned focus key changes.
    pub fn configure_projection_and_focus_changed_optional<Key: PartialEq>(
        &mut self,
        follow_state: &mut VirtualListFollowState<Key>,
        projection: VirtualListProjection,
        focus: VirtualListFocusTarget<Key>,
    ) -> VirtualListWindow {
        self.configure_projection(projection);
        if follow_state.update_focus_key(focus.key) {
            return match focus.index {
                Some(index) => self.focus(index),
                None => {
                    self.clear_focus();
                    self.resolve()
                }
            };
        }
        self.clear_focus();
        self.resolve()
    }

    /// Configure stable geometry inputs and follow focus with one adjacent
    /// context row before the guard band triggers scrolling.
    ///
    /// This is useful for dense browser, outline, table, and picker lists where
    /// selection-follow should preserve a small amount of context around the
    /// focused item instead of pinning it directly to a viewport edge.
    pub fn configure_and_focus_optional_with_context_row(
        &mut self,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        focused_index: Option<usize>,
    ) -> VirtualListWindow {
        self.configure_projection_and_focus_optional(
            VirtualListProjection::new(total_items, viewport_len, overscan, guard_band)
                .with_context_row(),
            focused_index,
        )
    }

    /// Configure stable geometry inputs and follow changed focus with one
    /// adjacent context row before guard-band scrolling.
    pub fn configure_and_focus_changed_optional_with_context_row<Key: PartialEq>(
        &mut self,
        follow_state: &mut VirtualListFollowState<Key>,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        focus: VirtualListFocusTarget<Key>,
    ) -> VirtualListWindow {
        self.configure_projection_and_focus_changed_optional(
            follow_state,
            VirtualListProjection::new(total_items, viewport_len, overscan, guard_band)
                .with_context_row(),
            focus,
        )
    }

    /// Configure named projection inputs and follow a key resolved from a slice
    /// only when an app-owned focus key changes.
    pub fn configure_slice_focus_changed_optional<Key: PartialEq>(
        &mut self,
        follow_state: &mut VirtualListFollowState<Key>,
        slice_focus: VirtualListSliceFocus<Key>,
    ) -> VirtualListWindow {
        self.configure_projection_and_focus_changed_optional(
            follow_state,
            slice_focus.projection(),
            slice_focus.focus,
        )
    }

    /// Clear focused-index anchoring.
    pub fn clear_focus(&mut self) {
        self.focused_index = None;
    }

    /// Set the focused item index and adjust the viewport if needed.
    pub fn focus(&mut self, index: usize) -> VirtualListWindow {
        self.focused_index = (index < self.total_items).then_some(index);
        self.resolve()
    }

    /// Set an optional focused item and adjust the viewport if needed.
    ///
    /// This is useful when the host selection is optional or app-owned. Invalid
    /// indices are treated the same as no focus.
    pub fn focus_optional(&mut self, index: Option<usize>) -> VirtualListWindow {
        self.focused_index = index.filter(|index| *index < self.total_items);
        self.resolve()
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualListController;

    #[test]
    fn focus_optional_follows_selection_with_guard_band() {
        let mut controller = VirtualListController::with_items(20, 6);
        controller.set_overscan(1);
        controller.set_guard_band(2);

        let window = controller.focus_optional(Some(4));

        assert_eq!(window.viewport_start, 1);
        assert_eq!(controller.viewport_start(), 1);
        assert_eq!(controller.focused_index(), Some(4));
    }

    #[test]
    fn focus_optional_with_context_row_expands_guard_band() {
        let mut controller = VirtualListController::with_items(20, 6);
        controller.set_viewport_start(0);

        let window = controller.configure_and_focus_optional_with_context_row(20, 6, 1, 1, Some(5));

        assert_eq!(window.viewport_start, 2);
        assert_eq!(controller.viewport_start(), 2);
        assert_eq!(controller.guard_band(), 2);
        assert_eq!(controller.focused_index(), Some(5));
    }
}
