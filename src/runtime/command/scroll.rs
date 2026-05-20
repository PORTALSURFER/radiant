use crate::layout::NodeId;

/// Named fields for revealing a vertical span inside one scroll container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollIntoViewParts {
    /// Scroll container node to move.
    pub node_id: NodeId,
    /// Logical top edge of the target span inside the scroll content.
    pub target_y: f32,
    /// Logical height of the target span.
    pub target_height: f32,
    /// Preferred space to keep above the target.
    pub margin_top: f32,
    /// Preferred space to keep below the target.
    pub margin_bottom: f32,
    /// Optional vertical snap interval for fixed-row lists.
    pub snap_y: Option<f32>,
}

/// Named fields for revealing a fixed-stride row inside one scroll container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollFixedRowIntoViewParts {
    /// Scroll container node to move.
    pub node_id: NodeId,
    /// Zero-based row index inside the scroll content.
    pub row_index: usize,
    /// Fixed distance between adjacent row starts in logical pixels.
    pub row_stride: f32,
    /// Rows to keep above the target while navigating upward.
    pub leading_context_rows: usize,
    /// Rows to keep below the target while navigating downward.
    pub trailing_context_rows: usize,
    /// Negative for upward navigation, positive for downward navigation.
    pub direction: i32,
}
