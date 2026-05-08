//! Generic virtual-list primitives.

use crate::gui::types::{Point, Rect};

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
