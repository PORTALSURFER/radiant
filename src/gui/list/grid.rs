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
/// allocating. Focus-follow behavior mirrors list windowing, but operates on
/// rows derived from the focused item index.
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
