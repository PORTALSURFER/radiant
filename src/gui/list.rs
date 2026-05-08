//! Generic list and row state primitives.

mod editable;

use crate::gui::types::{Point, Rect};

pub use editable::{ColumnSummary, EditableRowKind, EditableTreeActions, EditableTreeRow};

/// Request used to resolve a materialized window for a large logical list.
///
/// The request is item-index based rather than pixel based so host applications
/// can reuse it before projecting widgets or layout nodes. Pixel-based scroll
/// containers should continue to use `layout::VirtualizationPolicy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListWindowRequest {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of logical items visible in the viewport.
    pub viewport_len: usize,
    /// Host-requested viewport start.
    pub requested_start: usize,
    /// Extra logical items materialized before and after the viewport.
    pub overscan: usize,
    /// Optional focused item that should stay visible.
    pub focused_index: Option<usize>,
    /// Previously resolved viewport start, used to keep interior focus stable.
    pub previous_start: Option<usize>,
    /// Distance from the viewport edge that triggers focus-follow scrolling.
    pub guard_band: usize,
}

/// Resolved logical window for a virtualized list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListWindow {
    /// Total logical item count.
    pub total_items: usize,
    /// Start of the visible viewport.
    pub viewport_start: usize,
    /// End of the visible viewport, exclusive.
    pub viewport_end: usize,
    /// Start of the materialized window including overscan.
    pub window_start: usize,
    /// End of the materialized window including overscan, exclusive.
    pub window_end: usize,
}

impl VirtualListWindow {
    /// Number of materialized items in this window.
    pub fn window_len(self) -> usize {
        self.window_end.saturating_sub(self.window_start)
    }

    /// Number of visible viewport items in this window.
    pub fn viewport_len(self) -> usize {
        self.viewport_end.saturating_sub(self.viewport_start)
    }

    /// Return whether the materialized window contains no items.
    pub fn is_empty(self) -> bool {
        self.window_start == self.window_end
    }

    /// Return whether a logical item index is inside the materialized window.
    pub fn contains(self, index: usize) -> bool {
        index >= self.window_start && index < self.window_end
    }
}

/// Resolve an item-index based virtualized list window.
///
/// The algorithm is O(1), clamps every caller-provided bound, and avoids
/// allocating. When `focused_index` is present, the previous viewport start is
/// reused while the focus remains away from the configured guard band; near an
/// edge, the viewport scrolls just enough to keep focus comfortably visible.
pub fn resolve_virtual_list_window(request: VirtualListWindowRequest) -> VirtualListWindow {
    if request.total_items == 0 || request.viewport_len == 0 {
        return VirtualListWindow {
            total_items: request.total_items,
            ..VirtualListWindow::default()
        };
    }

    let viewport_len = request.viewport_len.min(request.total_items);
    let max_start = request.total_items.saturating_sub(viewport_len);
    let mut viewport_start = request
        .previous_start
        .unwrap_or(request.requested_start)
        .min(max_start);

    if let Some(focused_index) = request
        .focused_index
        .filter(|index| *index < request.total_items)
    {
        let guard_band = request.guard_band.min(viewport_len.saturating_sub(1) / 2);
        let viewport_end = viewport_start + viewport_len;
        let lower_guard = viewport_start + guard_band;
        let upper_guard_exclusive = viewport_end.saturating_sub(guard_band);

        if focused_index < lower_guard {
            viewport_start = focused_index.saturating_sub(guard_band).min(max_start);
        } else if focused_index >= upper_guard_exclusive {
            viewport_start = focused_index
                .saturating_add(guard_band + 1)
                .saturating_sub(viewport_len)
                .min(max_start);
        }
    } else {
        viewport_start = request.requested_start.min(max_start);
    }

    let viewport_end = viewport_start + viewport_len;
    let window_start = viewport_start.saturating_sub(request.overscan);
    let window_end = viewport_end
        .saturating_add(request.overscan)
        .min(request.total_items);

    VirtualListWindow {
        total_items: request.total_items,
        viewport_start,
        viewport_end,
        window_start,
        window_end,
    }
}

/// Apply a signed logical-row scroll delta to a virtual list viewport start.
///
/// This helper is O(1), allocation-free, and clamps the result to the current
/// visible item range. It is intentionally input-backend agnostic: native
/// runtimes can map wheel/touchpad/key input into `delta`, while hosts keep
/// ownership of hit testing and domain-specific scroll actions.
pub fn virtual_list_view_start_after_scroll_delta(
    current_view_start: usize,
    total_items: usize,
    viewport_len: usize,
    delta: isize,
) -> Option<usize> {
    if total_items == 0 || viewport_len == 0 || delta == 0 {
        return None;
    }
    let max_start = total_items.saturating_sub(viewport_len.min(total_items));
    let target = (current_view_start as isize + delta).clamp(0, max_start as isize);
    Some(target as usize)
}

/// Convert signed logical scroll units into a bounded virtual-list row delta.
///
/// `raw_units` should already be normalized by the caller: for example,
/// platform line deltas can be passed directly and pixel deltas can be divided
/// by a row stride. Any nonzero sub-row movement rounds to one row in the same
/// direction so high-resolution touchpads remain responsive.
pub fn virtual_list_scroll_delta_from_units(raw_units: f32) -> Option<i8> {
    if raw_units == 0.0 {
        return None;
    }
    let mut steps = raw_units.round();
    if steps.abs() < 1.0 {
        steps = raw_units.signum();
        if steps == 0.0 {
            return None;
        }
    }
    if steps == 0.0 {
        return None;
    }
    let clamped = if steps > 1.0 {
        steps.min(i8::MAX as f32)
    } else {
        steps.max(i8::MIN as f32)
    };
    Some(clamped as i8)
}

/// Stable host-supplied identity for one virtual-list item.
///
/// The key is intentionally opaque to Radiant. Hosts can derive it from an ID,
/// path hash, database row id, or any other stable origin while Radiant keeps
/// retained windows and state overlays independent from domain concepts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualListItemKey(pub u64);

/// Generic visual state carried by one materialized list item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListItemState {
    /// The item is part of the current selection.
    pub selected: bool,
    /// The item owns keyboard focus or caret state.
    pub focused: bool,
    /// The pointer is currently hovering the item.
    pub hovered: bool,
    /// The item is the current active drag/drop or operation target.
    pub active_target: bool,
    /// The item should render with disabled/unavailable treatment.
    pub disabled: bool,
    /// Transient operation state for retained row overlays.
    pub overlay: VirtualListItemOverlay,
}

/// Domain-neutral retained item overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VirtualListItemOverlay {
    /// No transient overlay is active.
    #[default]
    None,
    /// The item is waiting for a host-defined operation.
    Queued,
    /// The item is actively being processed by a host-defined operation.
    Active,
    /// The last host-defined operation completed successfully.
    Completed,
    /// The last host-defined operation failed.
    Failed,
}

impl VirtualListItemState {
    /// Return whether this state needs a retained overlay segment.
    pub fn requires_overlay(self) -> bool {
        self.selected
            || self.focused
            || self.hovered
            || self.active_target
            || self.disabled
            || self.overlay != VirtualListItemOverlay::None
    }
}

/// One materialized virtual-list item with stable identity and window geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterializedVirtualListItem {
    /// Stable host-supplied item identity.
    pub key: VirtualListItemKey,
    /// Logical item index in the full list.
    pub index: usize,
    /// Window-space row bounds.
    pub rect: Rect,
    /// Domain-neutral retained item state.
    pub state: VirtualListItemState,
}

impl MaterializedVirtualListItem {
    /// Build one materialized item.
    pub fn new(key: VirtualListItemKey, index: usize, rect: Rect) -> Self {
        Self {
            key,
            index,
            rect,
            state: VirtualListItemState::default(),
        }
    }

    /// Attach retained item state.
    pub fn with_state(mut self, state: VirtualListItemState) -> Self {
        self.state = state;
        self
    }
}

/// Axis-aligned metrics for a stacked virtual list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListStackMetrics {
    /// Logical item extent on the scrolling axis.
    pub item_extent: f32,
    /// Logical gap between adjacent items.
    pub item_gap: f32,
    /// Optional cap for visible items in one viewport.
    pub max_viewport_len: Option<usize>,
}

impl VirtualListStackMetrics {
    /// Build normalized stacked-list metrics.
    pub fn new(item_extent: f32, item_gap: f32) -> Self {
        Self {
            item_extent: item_extent.max(1.0),
            item_gap: item_gap.max(0.0),
            max_viewport_len: None,
        }
    }

    /// Apply a maximum viewport length cap.
    pub fn with_max_viewport_len(mut self, max_viewport_len: usize) -> Self {
        self.max_viewport_len = Some(max_viewport_len.max(1));
        self
    }

    /// Return the scrolling-axis stride between adjacent items.
    pub fn stride(self) -> f32 {
        (self.item_extent + self.item_gap).max(1.0)
    }
}

/// Resolve the number of stacked items visible in a viewport extent.
pub fn virtual_list_viewport_len_for_extent(
    viewport_extent: f32,
    metrics: VirtualListStackMetrics,
) -> usize {
    let geometric = ((viewport_extent.max(0.0) + metrics.item_gap) / metrics.stride())
        .floor()
        .max(1.0) as usize;
    metrics
        .max_viewport_len
        .map_or(geometric, |limit| geometric.min(limit))
        .max(1)
}

/// Resolve one item index from stacked virtual-list row geometry in O(1).
pub fn virtual_list_stacked_item_at_point(
    items: &[MaterializedVirtualListItem],
    point: Point,
) -> Option<usize> {
    let first = items.first()?;
    if point.x < first.rect.min.x || point.x > first.rect.max.x {
        return None;
    }

    let item_extent = first.rect.height().max(0.0);
    let stride = if items.len() > 1 {
        (items[1].rect.min.y - first.rect.min.y).max(1.0)
    } else {
        item_extent.max(1.0)
    };
    let relative_y = point.y - first.rect.min.y;
    if relative_y < 0.0 {
        return None;
    }

    let candidate = (relative_y / stride).floor() as usize;
    if candidate >= items.len() {
        return None;
    }
    let item_start = first.rect.min.y + (candidate as f32 * stride);
    let item_end = item_start + item_extent;
    if candidate > 0 {
        let previous_end = item_start - stride + item_extent;
        if point.y <= previous_end {
            return Some(items[candidate - 1].index);
        }
    }
    ((point.y >= item_start) && (point.y <= item_end)).then_some(items[candidate].index)
}

/// Request used to resolve one generic virtual-list scrollbar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListScrollbarRequest {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Total logical item count.
    pub total_items: usize,
    /// Visible viewport item count.
    pub viewport_len: usize,
    /// Current viewport start.
    pub viewport_start: usize,
    /// Minimum thumb extent on the scrolling axis.
    pub min_thumb_extent: f32,
}

/// Resolved scrollbar geometry for one virtual-list viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualListScrollbar {
    /// Scrollbar track in window coordinates.
    pub track: Rect,
    /// Scrollbar thumb in window coordinates.
    pub thumb: Rect,
}

/// Resolve vertical scrollbar geometry for a virtual list.
pub fn resolve_virtual_list_scrollbar(
    request: VirtualListScrollbarRequest,
) -> Option<VirtualListScrollbar> {
    if request.total_items == 0
        || request.viewport_len == 0
        || request.total_items <= request.viewport_len
        || request.track.height() <= 1.0
        || request.track.width() <= 0.0
    {
        return None;
    }

    let viewport_len = request.viewport_len.min(request.total_items);
    let max_viewport_start = request.total_items.saturating_sub(viewport_len);
    let thumb_extent = (request.track.height()
        * (viewport_len as f32 / request.total_items as f32))
        .round()
        .clamp(request.min_thumb_extent.max(1.0), request.track.height());
    let travel = (request.track.height() - thumb_extent).max(0.0);
    let start_ratio = if max_viewport_start == 0 {
        0.0
    } else {
        request.viewport_start.min(max_viewport_start) as f32 / max_viewport_start as f32
    };
    let thumb_min_y = (request.track.min.y + travel * start_ratio).round();
    let thumb = Rect::from_min_max(
        Point::new(request.track.min.x, thumb_min_y),
        Point::new(
            request.track.max.x,
            (thumb_min_y + thumb_extent).min(request.track.max.y),
        ),
    );

    Some(VirtualListScrollbar {
        track: request.track,
        thumb,
    })
}

/// Resolve a virtual-list viewport start from a dragged scrollbar thumb.
pub fn virtual_list_scrollbar_view_start_for_pointer(
    scrollbar: VirtualListScrollbar,
    viewport_len: usize,
    total_items: usize,
    pointer_y: f32,
    thumb_pointer_offset_y: f32,
) -> Option<usize> {
    if viewport_len == 0 || total_items <= viewport_len {
        return None;
    }
    let max_viewport_start = total_items.saturating_sub(viewport_len);
    let thumb_extent = scrollbar.thumb.height().max(1.0);
    let travel = (scrollbar.track.height() - thumb_extent).max(0.0);
    if travel <= f32::EPSILON || max_viewport_start == 0 {
        return Some(0);
    }

    let thumb_min_y = (pointer_y - thumb_pointer_offset_y)
        .clamp(scrollbar.track.min.y, scrollbar.track.max.y - thumb_extent);
    let start_ratio = ((thumb_min_y - scrollbar.track.min.y) / travel).clamp(0.0, 1.0);
    Some(((start_ratio * max_viewport_start as f32).round() as usize).min(max_viewport_start))
}

/// Retained-list invalidation summary for bounded rebuild decisions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListInvalidation {
    /// Logical item order, count, or identity changed.
    pub structure_changed: bool,
    /// The materialized viewport/window changed.
    pub window_changed: bool,
    /// One or more materialized item bounds changed.
    pub geometry_changed: bool,
    /// One or more item visual states changed.
    pub item_state_changed: bool,
}

impl VirtualListInvalidation {
    /// Return whether materialized item geometry must be rebuilt.
    pub fn requires_geometry_rebuild(self) -> bool {
        self.structure_changed || self.window_changed || self.geometry_changed
    }

    /// Return whether retained state overlays must be rebuilt.
    pub fn requires_overlay_rebuild(self) -> bool {
        self.requires_geometry_rebuild() || self.item_state_changed
    }
}

/// Request used to resolve a materialized row window for a large logical grid.
///
/// The request is item-index based and assumes a dense row-major grid. Hosts can
/// resolve the window before projecting card/tile widgets so large grids avoid
/// allocating or rebuilding off-screen rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualGridWindowRequest {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of grid columns in the current viewport.
    pub columns: usize,
    /// Number of visible grid rows in the viewport.
    pub viewport_rows: usize,
    /// Host-requested viewport start row.
    pub requested_row: usize,
    /// Extra rows materialized before and after the viewport.
    pub overscan_rows: usize,
    /// Optional focused item that should stay visible.
    pub focused_index: Option<usize>,
    /// Previously resolved viewport start row, used to keep interior focus stable.
    pub previous_row: Option<usize>,
    /// Distance from the viewport row edge that triggers focus-follow scrolling.
    pub guard_rows: usize,
}

/// Resolved logical row and item window for a virtualized dense grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualGridWindow {
    /// Total logical item count.
    pub total_items: usize,
    /// Number of grid columns used to resolve row boundaries.
    pub columns: usize,
    /// Total logical row count.
    pub total_rows: usize,
    /// Start of the visible viewport row window.
    pub viewport_row_start: usize,
    /// End of the visible viewport row window, exclusive.
    pub viewport_row_end: usize,
    /// Start of the materialized row window including overscan.
    pub window_row_start: usize,
    /// End of the materialized row window including overscan, exclusive.
    pub window_row_end: usize,
    /// First materialized item index.
    pub item_start: usize,
    /// End of materialized item indices, exclusive.
    pub item_end: usize,
}

impl VirtualGridWindow {
    /// Number of materialized rows in this window.
    pub fn window_row_len(self) -> usize {
        self.window_row_end.saturating_sub(self.window_row_start)
    }

    /// Number of visible viewport rows in this window.
    pub fn viewport_row_len(self) -> usize {
        self.viewport_row_end
            .saturating_sub(self.viewport_row_start)
    }

    /// Number of materialized items in this window.
    pub fn item_len(self) -> usize {
        self.item_end.saturating_sub(self.item_start)
    }

    /// Return whether the materialized window contains no items.
    pub fn is_empty(self) -> bool {
        self.item_start == self.item_end
    }

    /// Return whether a logical item index is inside the materialized window.
    pub fn contains_item(self, index: usize) -> bool {
        index >= self.item_start && index < self.item_end
    }
}

/// Resolve an item-index based virtualized grid window.
///
/// The algorithm is O(1), clamps every caller-provided bound, and avoids
/// allocating. Focus-follow behavior mirrors [`resolve_virtual_list_window`],
/// but operates on rows derived from the focused item index.
pub fn resolve_virtual_grid_window(request: VirtualGridWindowRequest) -> VirtualGridWindow {
    if request.total_items == 0 || request.columns == 0 || request.viewport_rows == 0 {
        return VirtualGridWindow {
            total_items: request.total_items,
            columns: request.columns,
            ..VirtualGridWindow::default()
        };
    }

    let total_rows = request.total_items.div_ceil(request.columns);
    let viewport_rows = request.viewport_rows.min(total_rows);
    let max_row = total_rows.saturating_sub(viewport_rows);
    let mut viewport_row_start = request
        .previous_row
        .unwrap_or(request.requested_row)
        .min(max_row);

    if let Some(focused_index) = request
        .focused_index
        .filter(|index| *index < request.total_items)
    {
        let focused_row = focused_index / request.columns;
        let guard_rows = request.guard_rows.min(viewport_rows.saturating_sub(1) / 2);
        let viewport_row_end = viewport_row_start + viewport_rows;
        let lower_guard = viewport_row_start + guard_rows;
        let upper_guard_exclusive = viewport_row_end.saturating_sub(guard_rows);

        if focused_row < lower_guard {
            viewport_row_start = focused_row.saturating_sub(guard_rows).min(max_row);
        } else if focused_row >= upper_guard_exclusive {
            viewport_row_start = focused_row
                .saturating_add(guard_rows + 1)
                .saturating_sub(viewport_rows)
                .min(max_row);
        }
    } else {
        viewport_row_start = request.requested_row.min(max_row);
    }

    let viewport_row_end = viewport_row_start + viewport_rows;
    let window_row_start = viewport_row_start.saturating_sub(request.overscan_rows);
    let window_row_end = viewport_row_end
        .saturating_add(request.overscan_rows)
        .min(total_rows);
    let item_start = window_row_start.saturating_mul(request.columns);
    let item_end = window_row_end
        .saturating_mul(request.columns)
        .min(request.total_items);

    VirtualGridWindow {
        total_items: request.total_items,
        columns: request.columns,
        total_rows,
        viewport_row_start,
        viewport_row_end,
        window_row_start,
        window_row_end,
        item_start,
        item_end,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ColumnSummary, EditableRowKind, EditableTreeActions, EditableTreeRow,
        MaterializedVirtualListItem, VirtualGridWindow, VirtualGridWindowRequest,
        VirtualListInvalidation, VirtualListItemKey, VirtualListItemOverlay, VirtualListItemState,
        VirtualListScrollbarRequest, VirtualListStackMetrics, VirtualListWindow,
        VirtualListWindowRequest, resolve_virtual_grid_window, resolve_virtual_list_scrollbar,
        resolve_virtual_list_window, virtual_list_scroll_delta_from_units,
        virtual_list_scrollbar_view_start_for_pointer, virtual_list_stacked_item_at_point,
        virtual_list_view_start_after_scroll_delta, virtual_list_viewport_len_for_extent,
    };
    use crate::gui::types::{Point, Rect};

    #[test]
    fn column_summary_preserves_title_and_count() {
        let column = ColumnSummary::new("Inbox", 42);

        assert_eq!(column.title, "Inbox");
        assert_eq!(column.item_count, 42);
    }

    #[test]
    fn editable_row_kind_defaults_to_existing() {
        assert_eq!(EditableRowKind::default(), EditableRowKind::Existing);
    }

    #[test]
    fn editable_tree_actions_default_to_unavailable() {
        let actions = EditableTreeActions::default();

        assert!(!actions.can_create_child);
        assert!(!actions.can_create_root);
        assert!(!actions.can_rename);
        assert!(!actions.can_delete);
        assert!(!actions.can_restore_retained);
        assert!(!actions.can_purge_retained);
        assert!(!actions.can_clear_history);
    }

    #[test]
    fn editable_tree_row_preserves_existing_and_draft_state() {
        let existing = EditableTreeRow::new("Root", "3 items", 0, true, false, true, true, true)
            .with_backing_index(7);
        let draft = EditableTreeRow::rename_draft(1, "Draft", "Name", None, true);

        assert_eq!(existing.label, "Root");
        assert_eq!(existing.detail, "3 items");
        assert_eq!(existing.kind, EditableRowKind::Existing);
        assert_eq!(existing.backing_index, Some(7));
        assert_eq!(draft.kind, EditableRowKind::RenameDraft);
        assert_eq!(draft.input_value.as_deref(), Some("Draft"));
        assert!(draft.input_focused);
        assert!(draft.select_all_on_focus);
    }

    #[test]
    fn virtual_list_window_clamps_requested_bounds_and_applies_overscan() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 100,
            viewport_len: 10,
            requested_start: 95,
            overscan: 3,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualListWindow {
                total_items: 100,
                viewport_start: 90,
                viewport_end: 100,
                window_start: 87,
                window_end: 100,
            }
        );
        assert_eq!(window.viewport_len(), 10);
        assert_eq!(window.window_len(), 13);
        assert!(window.contains(99));
        assert!(!window.contains(86));
    }

    #[test]
    fn virtual_list_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(310),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(window.viewport_start, 300);
        assert_eq!(window.viewport_end, 320);
    }

    #[test]
    fn virtual_list_window_scrolls_when_focus_reaches_guard_band() {
        let top = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(302),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });
        let bottom = resolve_virtual_list_window(VirtualListWindowRequest {
            total_items: 1_000,
            viewport_len: 20,
            requested_start: 300,
            previous_start: Some(300),
            focused_index: Some(318),
            guard_band: 4,
            ..VirtualListWindowRequest::default()
        });

        assert_eq!(top.viewport_start, 298);
        assert_eq!(bottom.viewport_start, 303);
    }

    #[test]
    fn virtual_list_window_handles_empty_or_zero_viewport_requests() {
        assert!(resolve_virtual_list_window(VirtualListWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_list_window(VirtualListWindowRequest {
                total_items: 10,
                viewport_len: 0,
                ..VirtualListWindowRequest::default()
            })
            .is_empty()
        );
    }

    #[test]
    fn virtual_list_scroll_delta_clamps_to_visible_bounds() {
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(10, 40, 12, -3),
            Some(7)
        );
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(0, 40, 12, -3),
            Some(0)
        );
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(27, 40, 12, 5),
            Some(28)
        );
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(4, 0, 12, 2),
            None
        );
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(4, 20, 0, 2),
            None
        );
        assert_eq!(
            virtual_list_view_start_after_scroll_delta(4, 20, 12, 0),
            None
        );
    }

    #[test]
    fn virtual_list_scroll_delta_from_units_rounds_and_clamps_steps() {
        assert_eq!(virtual_list_scroll_delta_from_units(0.0), None);
        assert_eq!(virtual_list_scroll_delta_from_units(0.2), Some(1));
        assert_eq!(virtual_list_scroll_delta_from_units(-0.2), Some(-1));
        assert_eq!(virtual_list_scroll_delta_from_units(3.4), Some(3));
        assert_eq!(virtual_list_scroll_delta_from_units(-3.6), Some(-4));
        assert_eq!(virtual_list_scroll_delta_from_units(400.0), Some(i8::MAX));
        assert_eq!(virtual_list_scroll_delta_from_units(-400.0), Some(i8::MIN));
    }

    #[test]
    fn virtual_list_viewport_len_uses_geometry_and_caps_capacity() {
        let metrics = VirtualListStackMetrics::new(24.0, 4.0).with_max_viewport_len(6);

        assert_eq!(virtual_list_viewport_len_for_extent(0.0, metrics), 1);
        assert_eq!(virtual_list_viewport_len_for_extent(139.0, metrics), 5);
        assert_eq!(virtual_list_viewport_len_for_extent(10_000.0, metrics), 6);
    }

    #[test]
    fn virtual_list_hit_testing_returns_stable_logical_indices() {
        let items = [
            MaterializedVirtualListItem::new(
                VirtualListItemKey(41),
                10,
                Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0)),
            ),
            MaterializedVirtualListItem::new(
                VirtualListItemKey(42),
                11,
                Rect::from_min_max(Point::new(10.0, 44.0), Point::new(110.0, 64.0)),
            ),
            MaterializedVirtualListItem::new(
                VirtualListItemKey(43),
                12,
                Rect::from_min_max(Point::new(10.0, 68.0), Point::new(110.0, 88.0)),
            ),
        ];

        assert_eq!(
            virtual_list_stacked_item_at_point(&items, Point::new(20.0, 45.0)),
            Some(11)
        );
        assert_eq!(
            virtual_list_stacked_item_at_point(&items, Point::new(20.0, 42.0)),
            None
        );
        assert_eq!(
            virtual_list_stacked_item_at_point(&items, Point::new(120.0, 45.0)),
            None
        );
    }

    #[test]
    fn virtual_list_scrollbar_maps_viewport_and_pointer_drag() {
        let track = Rect::from_min_max(Point::new(190.0, 10.0), Point::new(198.0, 210.0));
        let scrollbar = resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
            track,
            total_items: 100,
            viewport_len: 20,
            viewport_start: 40,
            min_thumb_extent: 18.0,
        })
        .expect("overflowing list has scrollbar");

        assert_eq!(scrollbar.track, track);
        assert_eq!(scrollbar.thumb.height(), 40.0);
        assert_eq!(scrollbar.thumb.min.y, 90.0);
        assert_eq!(
            virtual_list_scrollbar_view_start_for_pointer(scrollbar, 20, 100, 170.0, 20.0),
            Some(70)
        );
        assert_eq!(
            resolve_virtual_list_scrollbar(VirtualListScrollbarRequest {
                track,
                total_items: 10,
                viewport_len: 10,
                viewport_start: 0,
                min_thumb_extent: 18.0,
            }),
            None
        );
    }

    #[test]
    fn virtual_list_item_state_and_invalidation_are_overlay_oriented() {
        let idle = VirtualListItemState::default();
        let active = VirtualListItemState {
            selected: false,
            focused: true,
            hovered: false,
            active_target: false,
            disabled: false,
            overlay: VirtualListItemOverlay::Active,
        };
        let item = MaterializedVirtualListItem::new(
            VirtualListItemKey(9),
            3,
            Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 20.0)),
        )
        .with_state(active);
        let state_only = VirtualListInvalidation {
            item_state_changed: true,
            ..VirtualListInvalidation::default()
        };

        assert!(!idle.requires_overlay());
        assert!(item.state.requires_overlay());
        assert!(!state_only.requires_geometry_rebuild());
        assert!(state_only.requires_overlay_rebuild());
        assert!(
            VirtualListInvalidation {
                window_changed: true,
                ..VirtualListInvalidation::default()
            }
            .requires_geometry_rebuild()
        );
    }

    #[test]
    fn virtual_grid_window_clamps_rows_and_applies_overscan() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 103,
            columns: 5,
            viewport_rows: 4,
            requested_row: 99,
            overscan_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(
            window,
            VirtualGridWindow {
                total_items: 103,
                columns: 5,
                total_rows: 21,
                viewport_row_start: 17,
                viewport_row_end: 21,
                window_row_start: 15,
                window_row_end: 21,
                item_start: 75,
                item_end: 103,
            }
        );
        assert_eq!(window.viewport_row_len(), 4);
        assert_eq!(window.window_row_len(), 6);
        assert_eq!(window.item_len(), 28);
        assert!(window.contains_item(102));
        assert!(!window.contains_item(74));
    }

    #[test]
    fn virtual_grid_window_keeps_interior_focus_stable() {
        let window = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(178),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(window.viewport_row_start, 40);
        assert_eq!(window.viewport_row_end, 50);
    }

    #[test]
    fn virtual_grid_window_scrolls_when_focus_reaches_guard_row() {
        let top = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(164),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });
        let bottom = resolve_virtual_grid_window(VirtualGridWindowRequest {
            total_items: 1_000,
            columns: 4,
            viewport_rows: 10,
            requested_row: 40,
            previous_row: Some(40),
            focused_index: Some(192),
            guard_rows: 2,
            ..VirtualGridWindowRequest::default()
        });

        assert_eq!(top.viewport_row_start, 39);
        assert_eq!(bottom.viewport_row_start, 41);
    }

    #[test]
    fn virtual_grid_window_handles_empty_zero_column_or_zero_viewport_requests() {
        assert!(resolve_virtual_grid_window(VirtualGridWindowRequest::default()).is_empty());
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 0,
                viewport_rows: 2,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
        assert!(
            resolve_virtual_grid_window(VirtualGridWindowRequest {
                total_items: 10,
                columns: 3,
                viewport_rows: 0,
                ..VirtualGridWindowRequest::default()
            })
            .is_empty()
        );
    }
}
