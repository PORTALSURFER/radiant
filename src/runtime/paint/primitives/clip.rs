use crate::{gui::types::Rect, layout::NodeId};

/// Begin clipping subsequent paint primitives to a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipStart {
    /// Container node that owns this clip.
    pub node_id: NodeId,
    /// Clip rectangle in logical surface coordinates.
    pub rect: Rect,
}

/// End the most recent paint clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintClipEnd {
    /// Container node that owns the matching clip.
    pub node_id: NodeId,
}
