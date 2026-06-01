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

    /// Return a cursor advanced past one item and the following gap.
    ///
    /// This is the chainable form of [`Self::advance`] for compact overlay
    /// anchor calculations.
    pub fn advanced(mut self, item_extent: f32, gap_after: f32) -> Self {
        self.advance(item_extent, gap_after);
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
