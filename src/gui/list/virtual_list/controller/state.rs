/// Durable state for one item-index based virtual list viewport.
///
/// Store one controller per scrollable list surface. The controller owns only
/// viewport/focus-follow state for that list, so scrolling or focusing one
/// list does not invalidate another list unless the host explicitly shares
/// state between them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListController {
    pub(super) total_items: usize,
    pub(super) viewport_len: usize,
    pub(super) runtime_viewport_len: Option<usize>,
    pub(super) viewport_start: usize,
    pub(super) overscan: usize,
    pub(super) guard_band: usize,
    pub(super) focused_index: Option<usize>,
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
            runtime_viewport_len: None,
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

    /// Return the runtime-reported viewport length, if a scroll container has
    /// reported one.
    pub const fn runtime_viewport_len(&self) -> Option<usize> {
        self.runtime_viewport_len
    }

    /// Return the runtime-reported viewport length, falling back to host
    /// projection input.
    pub const fn runtime_viewport_len_or(&self, fallback: usize) -> usize {
        match self.runtime_viewport_len {
            Some(viewport_len) => viewport_len,
            None => fallback,
        }
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

    pub(super) fn clamp_viewport_start(&mut self) {
        let viewport_len = self.viewport_len.min(self.total_items);
        let max_start = self.total_items.saturating_sub(viewport_len);
        self.viewport_start = self.viewport_start.min(max_start);
    }
}
