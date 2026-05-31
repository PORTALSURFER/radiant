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

    /// Advance past one item and the following gap.
    pub fn advance(&mut self, item_extent: f32, gap_after: f32) {
        self.offset += finite_nonnegative(item_extent) + finite_nonnegative(gap_after);
    }
}

fn finite_nonnegative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
