use super::{
    VirtualListScrollbar, VirtualListScrollbarRequest, VirtualListWindow, VirtualListWindowRequest,
    resolve_virtual_list_scrollbar, resolve_virtual_list_window,
    virtual_list_scroll_delta_from_units, virtual_list_scrollbar_view_start_for_pointer,
    virtual_list_view_start_after_scroll_delta, virtual_list_view_start_for_scroll_offset,
};
use crate::gui::types::Rect;

/// Durable state for one item-index based virtual list viewport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListController {
    total_items: usize,
    viewport_len: usize,
    viewport_start: usize,
    overscan: usize,
    guard_band: usize,
    focused_index: Option<usize>,
}

/// Stable geometry inputs for one virtual-list projection pass.
///
/// Use this when the host already knows the logical item count, projected
/// viewport length, materialization overscan, and focus-follow guard band. The
/// named fields keep large-list projection call sites readable when several
/// virtualized panes share the same controller workflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualListProjection {
    total_items: usize,
    viewport_len: usize,
    overscan: usize,
    guard_band: usize,
}

/// App-owned focus key memory for virtual lists that should follow selection
/// only when the selected item changes.
///
/// Pair this with [`VirtualListController::configure_and_focus_changed_optional`]
/// when manual scroll should remain authoritative until the app selection moves
/// to a different key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListFollowState<Key> {
    focus_key: Option<Key>,
}

/// App-owned focus key and current item index for a virtualized list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListFocusTarget<Key> {
    /// Stable app-owned key for the focused item.
    pub key: Option<Key>,
    /// Current visible item index for the same key.
    pub index: Option<usize>,
}

/// Projection inputs and key-resolved focus for a current item slice.
///
/// Build this before mutably borrowing a [`VirtualListController`] when the
/// current projection slice borrows from the same host state as the controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListSliceFocus<Key> {
    projection: VirtualListProjection,
    focus: VirtualListFocusTarget<Key>,
}

impl<Key> VirtualListFocusTarget<Key> {
    /// Build a focus target from an app-owned key and current item index.
    pub const fn new(key: Option<Key>, index: Option<usize>) -> Self {
        Self { key, index }
    }

    /// Build an empty focus target.
    pub const fn none() -> Self {
        Self {
            key: None,
            index: None,
        }
    }
}

impl<Key: Clone> VirtualListFocusTarget<Key> {
    /// Build a focus target by locating a stable key inside a projected item slice.
    ///
    /// This keeps app code focused on its domain item projection while Radiant
    /// owns the common "key plus current index" shape used by virtual-list
    /// selection-follow. If the key is missing from the current slice, the
    /// returned target is empty so follow state clears instead of anchoring to a
    /// stale item.
    pub fn from_slice_by<Item>(
        items: &[Item],
        key: Option<Key>,
        mut matches_key: impl FnMut(&Item, &Key) -> bool,
    ) -> Self {
        let Some(key) = key else {
            return Self::none();
        };
        let index = items.iter().position(|item| matches_key(item, &key));
        match index {
            Some(index) => Self::new(Some(key), Some(index)),
            None => Self::none(),
        }
    }
}

impl<Key> VirtualListSliceFocus<Key> {
    /// Build slice-focus inputs from named projection inputs and a focus target.
    pub const fn new(
        projection: VirtualListProjection,
        focus: VirtualListFocusTarget<Key>,
    ) -> Self {
        Self { projection, focus }
    }

    /// Return the projection inputs.
    pub const fn projection(&self) -> VirtualListProjection {
        self.projection
    }

    /// Return the key-resolved focus target.
    pub const fn focus(&self) -> &VirtualListFocusTarget<Key> {
        &self.focus
    }

    /// Add context rows to the focus-follow guard band.
    pub fn with_context_rows(self, context_rows: usize) -> Self {
        Self {
            projection: self.projection.with_context_rows(context_rows),
            ..self
        }
    }

    /// Add one context row to the focus-follow guard band.
    pub fn with_context_row(self) -> Self {
        self.with_context_rows(1)
    }
}

impl<Key: Clone> VirtualListSliceFocus<Key> {
    /// Build slice-focus inputs by locating a stable key inside the current item
    /// slice and deriving projection item count from that same slice.
    pub fn from_slice_by<Item>(
        items: &[Item],
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        key: Option<Key>,
        matches_key: impl FnMut(&Item, &Key) -> bool,
    ) -> Self {
        Self::new(
            VirtualListProjection::for_slice(items, viewport_len, overscan, guard_band),
            VirtualListFocusTarget::from_slice_by(items, key, matches_key),
        )
    }
}

impl VirtualListProjection {
    /// Build virtual-list projection inputs.
    pub const fn new(
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
    ) -> Self {
        Self {
            total_items,
            viewport_len,
            overscan,
            guard_band,
        }
    }

    /// Build virtual-list projection inputs from the current item slice.
    pub const fn for_slice<Item>(
        items: &[Item],
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
    ) -> Self {
        Self::new(items.len(), viewport_len, overscan, guard_band)
    }

    /// Add context rows to the focus-follow guard band.
    ///
    /// Browser, outline, table, and picker lists often want one or more rows of
    /// nearby context around a selected item before follow scrolling moves the
    /// viewport. This keeps that policy explicit at the projection call site.
    pub const fn with_context_rows(self, context_rows: usize) -> Self {
        Self {
            guard_band: self.guard_band.saturating_add(context_rows),
            ..self
        }
    }

    /// Add one context row to the focus-follow guard band.
    pub const fn with_context_row(self) -> Self {
        self.with_context_rows(1)
    }

    /// Return the total logical item count.
    pub const fn total_items(&self) -> usize {
        self.total_items
    }

    /// Return the visible logical item count.
    pub const fn viewport_len(&self) -> usize {
        self.viewport_len
    }

    /// Return the materialization overscan.
    pub const fn overscan(&self) -> usize {
        self.overscan
    }

    /// Return the focus-follow guard band.
    pub const fn guard_band(&self) -> usize {
        self.guard_band
    }
}

impl<Key> Default for VirtualListFollowState<Key> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Key> VirtualListFollowState<Key> {
    /// Build an empty follow state.
    pub const fn new() -> Self {
        Self { focus_key: None }
    }

    /// Return the last followed focus key.
    pub const fn focus_key(&self) -> Option<&Key> {
        self.focus_key.as_ref()
    }

    /// Clear the last followed focus key.
    pub fn clear(&mut self) {
        self.focus_key = None;
    }
}

impl<Key: PartialEq> VirtualListFollowState<Key> {
    fn update_focus_key(&mut self, focus_key: Option<Key>) -> bool {
        if self.focus_key == focus_key {
            return false;
        }
        self.focus_key = focus_key;
        true
    }
}

impl Default for VirtualListController {
    fn default() -> Self {
        Self::new()
    }
}

impl VirtualListController {
    /// Build an empty virtual-list controller.
    pub const fn new() -> Self {
        Self {
            total_items: 0,
            viewport_len: 0,
            viewport_start: 0,
            overscan: 0,
            guard_band: 0,
            focused_index: None,
        }
    }

    /// Build a controller with an item count and visible viewport length.
    pub fn with_items(total_items: usize, viewport_len: usize) -> Self {
        let mut controller = Self::new();
        controller.total_items = total_items;
        controller.viewport_len = viewport_len;
        controller.clamp_viewport_start();
        controller
    }

    /// Return the total logical item count.
    pub const fn total_items(&self) -> usize {
        self.total_items
    }

    /// Return the visible logical item count.
    pub const fn viewport_len(&self) -> usize {
        self.viewport_len
    }

    /// Return the current viewport start.
    pub const fn viewport_start(&self) -> usize {
        self.viewport_start
    }

    /// Return the materialization overscan.
    pub const fn overscan(&self) -> usize {
        self.overscan
    }

    /// Return the focus-follow guard band.
    pub const fn guard_band(&self) -> usize {
        self.guard_band
    }

    /// Return the focused item index, if any.
    pub const fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    /// Set the total logical item count and clamp dependent state.
    pub fn set_total_items(&mut self, total_items: usize) {
        self.total_items = total_items;
        if self
            .focused_index
            .is_some_and(|index| index >= self.total_items)
        {
            self.focused_index = None;
        }
        self.clamp_viewport_start();
    }

    /// Set the visible logical item count and clamp dependent state.
    pub fn set_viewport_len(&mut self, viewport_len: usize) {
        self.viewport_len = viewport_len;
        self.clamp_viewport_start();
    }

    /// Set the materialization overscan.
    pub fn set_overscan(&mut self, overscan: usize) {
        self.overscan = overscan;
    }

    /// Set the focus-follow guard band.
    pub fn set_guard_band(&mut self, guard_band: usize) {
        self.guard_band = guard_band;
    }

    /// Configure the stable geometry inputs for a projection pass.
    ///
    /// The current viewport start is clamped after the item and viewport counts
    /// are updated, so callers can safely reuse one controller while filters,
    /// sorts, window sizes, or overscan policy change.
    pub fn configure(
        &mut self,
        total_items: usize,
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
    ) {
        self.total_items = total_items;
        self.viewport_len = viewport_len;
        self.overscan = overscan;
        self.guard_band = guard_band;
        if self
            .focused_index
            .is_some_and(|index| index >= self.total_items)
        {
            self.focused_index = None;
        }
        self.clamp_viewport_start();
    }

    /// Configure the stable geometry inputs from a named projection value.
    pub fn configure_projection(&mut self, projection: VirtualListProjection) {
        self.configure(
            projection.total_items,
            projection.viewport_len,
            projection.overscan,
            projection.guard_band,
        );
    }

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
            slice_focus.projection,
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

    fn clamp_viewport_start(&mut self) {
        let viewport_len = self.viewport_len.min(self.total_items);
        let max_start = self.total_items.saturating_sub(viewport_len);
        self.viewport_start = self.viewport_start.min(max_start);
    }
}

#[cfg(test)]
mod tests {
    use super::VirtualListController;

    #[test]
    fn configure_clamps_viewport_and_invalid_focus() {
        let mut controller = VirtualListController::with_items(20, 6);
        controller.set_viewport_start(14);
        controller.focus(18);

        controller.configure(5, 3, 1, 1);

        assert_eq!(controller.total_items(), 5);
        assert_eq!(controller.viewport_len(), 3);
        assert_eq!(controller.overscan(), 1);
        assert_eq!(controller.guard_band(), 1);
        assert_eq!(controller.viewport_start(), 2);
        assert_eq!(controller.focused_index(), None);
    }

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
