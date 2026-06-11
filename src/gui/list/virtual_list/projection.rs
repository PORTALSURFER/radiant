/// Stable geometry inputs for one virtual-list projection pass.
///
/// Use this when the host already knows the logical item count, projected
/// viewport length, materialization overscan, and focus-follow guard band. The
/// named fields keep large-list projection call sites readable when several
/// virtualized panes share the same controller workflow.
///
/// Projection is intentionally count-based: construct row views after resolving
/// a [`super::VirtualListWindow`], and only for the materialized range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualListProjection {
    pub(super) total_items: usize,
    pub(super) viewport_len: usize,
    pub(super) overscan: usize,
    pub(super) guard_band: usize,
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
