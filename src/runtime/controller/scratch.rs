use crate::{gui::types::Vector2, layout::NodeId};

/// Reusable temporary buffers for runtime projection and layout synchronization.
#[derive(Default)]
pub(super) struct RuntimeScratch {
    pub(super) scroll_clamp_updates: Vec<(NodeId, Vector2)>,
    pub(super) projection_scroll_stack: Vec<NodeId>,
    pub(super) projection_child_path: Vec<usize>,
}
