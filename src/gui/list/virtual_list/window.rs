/// Request used to resolve a materialized window for a large logical list.
///
/// The request is item-index based rather than pixel based so host applications
/// can reuse it before projecting widgets or layout nodes. Pixel-based scroll
/// containers should continue to use `layout::VirtualizationPolicy`.
///
/// Hosts should treat this as the projection contract for large lists: build
/// rows only for the returned `window_start..window_end` range, keep the full
/// logical count as metadata, and use stable row keys for retained focus,
/// hover, selection, and overlay state.
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
///
/// `viewport_start..viewport_end` is the visible row range. The wider
/// `window_start..window_end` range is the only range that should be
/// materialized into row widgets, hit-test entries, or repaint overlays during
/// a normal scroll frame.
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

/// Runtime-originated fixed-row virtual-list window change.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VirtualListWindowChange {
    /// Pixel scroll offset reported by the scroll container.
    pub offset_y: f32,
    /// Fixed logical row height used to map pixels to logical rows.
    pub row_height: f32,
    /// Resolved logical window after applying the scroll offset.
    pub window: VirtualListWindow,
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

    /// Number of materialized overscan rows retained around the viewport.
    pub fn overscan(self) -> usize {
        self.leading_overscan().max(self.trailing_overscan())
    }

    /// Number of materialized rows retained before the viewport.
    pub fn leading_overscan(self) -> usize {
        self.viewport_start.saturating_sub(self.window_start)
    }

    /// Number of materialized rows retained after the viewport.
    pub fn trailing_overscan(self) -> usize {
        self.window_end.saturating_sub(self.viewport_end)
    }

    /// Return whether the materialized window contains no items.
    pub fn is_empty(self) -> bool {
        self.window_start == self.window_end
    }

    /// Return whether a logical item index is inside the visible viewport.
    pub fn viewport_contains(self, index: usize) -> bool {
        index >= self.viewport_start && index < self.viewport_end
    }

    /// Return whether a logical item index is inside the materialized window.
    pub fn contains(self, index: usize) -> bool {
        index >= self.window_start && index < self.window_end
    }

    /// Reconcile this resolved window against a new logical item count.
    ///
    /// Use this when host-owned list data changes after a window was cached for
    /// projection. The returned window preserves the current viewport start,
    /// visible length, and widest overscan policy while clamping every bound to
    /// the new item count. If the current window has no visible rows and the
    /// new count is nonzero, one visible row is used as the minimum recoverable
    /// viewport length; hosts with an authoritative viewport size can call
    /// [`resolve_virtual_list_window`] directly.
    pub fn reconcile_total_items(self, total_items: usize) -> Self {
        resolve_virtual_list_window(VirtualListWindowRequest {
            total_items,
            viewport_len: self.viewport_len().max(1),
            requested_start: self.viewport_start,
            overscan: self.overscan(),
            focused_index: None,
            previous_start: None,
            guard_band: 0,
        })
    }
}

/// Resolve an item-index based virtualized list window.
///
/// The algorithm is O(1), clamps every caller-provided bound, and avoids
/// allocating. When `focused_index` is present, the previous viewport start is
/// reused while the focus remains away from the configured guard band; near an
/// edge, the viewport scrolls just enough to keep focus comfortably visible.
/// The returned materialized length is bounded by
/// `viewport_len + overscan * 2`, capped by the logical item count.
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

/// Resolve a virtual-list viewport start from a logical scroll offset.
///
/// This is useful for hosts receiving pixel-based scroll positions from a
/// platform scroll container while their virtualized list model remains
/// item-index based. The offset is clamped to the current item range and the
/// row extent is floored at one logical unit to avoid division by zero.
pub fn virtual_list_view_start_for_scroll_offset(
    offset: f32,
    row_extent: f32,
    total_items: usize,
) -> usize {
    if total_items == 0 {
        return 0;
    }
    ((offset.max(0.0) / row_extent.max(1.0)).floor() as usize).min(total_items.saturating_sub(1))
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
