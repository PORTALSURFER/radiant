#[cfg(test)]
#[path = "stack_cursor/tests.rs"]
mod tests;

/// Accumulates offsets for one-dimensional stacked UI content.
///
/// Use this when overlay anchors or retained geometry need the same vertical or
/// horizontal spacing model as a compact stack, but constructing a full layout
/// tree would be unnecessary.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StackedLayoutCursor {
    offset: f32,
}

/// One fixed-size item in a one-dimensional stacked layout.
///
/// Use this with [`StackedLayoutCursor::from_items`] or
/// [`StackedLayoutCursor::advanced_items`] when an overlay anchor should mirror
/// a data-driven stack of rows without rebuilding the full layout tree.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StackedLayoutItem {
    extent: f32,
    gap_after: f32,
}

impl StackedLayoutItem {
    /// Create a stacked layout item from its extent and following gap.
    pub fn new(extent: f32, gap_after: f32) -> Self {
        Self {
            extent: finite_nonnegative(extent),
            gap_after: finite_nonnegative(gap_after),
        }
    }

    /// Return the item's finite non-negative extent.
    pub const fn extent(self) -> f32 {
        self.extent
    }

    /// Return the finite non-negative gap after the item.
    pub const fn gap_after(self) -> f32 {
        self.gap_after
    }
}

impl StackedLayoutCursor {
    /// Create a cursor at the start of a stack.
    pub const fn new() -> Self {
        Self { offset: 0.0 }
    }

    /// Create a cursor from an existing offset.
    pub fn from_offset(offset: f32) -> Self {
        Self {
            offset: finite_nonnegative(offset),
        }
    }

    /// Create a cursor advanced past the provided stacked items.
    pub fn from_items(items: impl IntoIterator<Item = StackedLayoutItem>) -> Self {
        Self::new().advanced_items(items)
    }

    /// Return the current offset from the start edge.
    pub const fn offset(self) -> f32 {
        self.offset
    }

    /// Return an offset inside the current item without advancing the cursor.
    ///
    /// Use this for overlay anchors or retained geometry that targets a nested
    /// control within the current stacked item, such as a label-over-control
    /// group whose popup should align with the control rather than the label.
    pub fn offset_within_item(self, item_offset: f32) -> f32 {
        self.offset + finite_nonnegative(item_offset)
    }

    /// Advance past one item and the following gap.
    pub fn advance(&mut self, item_extent: f32, gap_after: f32) {
        self.offset += finite_nonnegative(item_extent) + finite_nonnegative(gap_after);
    }

    /// Advance past one named stack item.
    pub fn advance_item(&mut self, item: StackedLayoutItem) {
        self.offset += item.extent() + item.gap_after();
    }

    /// Advance past several identical items and their following gaps.
    pub fn advance_many(&mut self, count: usize, item_extent: f32, gap_after: f32) {
        let step = finite_nonnegative(item_extent) + finite_nonnegative(gap_after);
        self.offset += step * count as f32;
    }

    /// Advance past several named stack items.
    pub fn advance_items(&mut self, items: impl IntoIterator<Item = StackedLayoutItem>) {
        for item in items {
            self.advance_item(item);
        }
    }

    /// Return a cursor advanced past one item and the following gap.
    ///
    /// This is the chainable form of [`Self::advance`] for compact overlay
    /// anchor calculations.
    pub fn advanced(mut self, item_extent: f32, gap_after: f32) -> Self {
        self.advance(item_extent, gap_after);
        self
    }

    /// Return a cursor advanced past one named stack item.
    pub fn advanced_item(mut self, item: StackedLayoutItem) -> Self {
        self.advance_item(item);
        self
    }

    /// Return a cursor advanced past several identical items and their gaps.
    pub fn advanced_many(mut self, count: usize, item_extent: f32, gap_after: f32) -> Self {
        self.advance_many(count, item_extent, gap_after);
        self
    }

    /// Return a cursor advanced past several named stack items.
    pub fn advanced_items(mut self, items: impl IntoIterator<Item = StackedLayoutItem>) -> Self {
        self.advance_items(items);
        self
    }

    /// Conditionally return a cursor advanced past one item and the following gap.
    ///
    /// Use this when an optional row affects retained geometry or overlay
    /// anchors and the app wants to keep the stack math declarative.
    pub fn advanced_if(self, condition: bool, item_extent: f32, gap_after: f32) -> Self {
        if condition {
            self.advanced(item_extent, gap_after)
        } else {
            self
        }
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
