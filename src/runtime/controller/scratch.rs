use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::surface::{DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY, ViewDeltaScratch},
};

/// Reusable temporary buffers for runtime projection and layout synchronization.
pub(super) struct RuntimeScratch {
    pub(super) scroll_clamp_updates: Vec<(NodeId, Vector2)>,
    pub(super) projection_scroll_stack: Vec<NodeId>,
    pub(super) projection_child_path: Vec<usize>,
    pub(super) view_delta: ViewDeltaScratch,
}

impl Default for RuntimeScratch {
    fn default() -> Self {
        Self {
            scroll_clamp_updates: Vec::new(),
            projection_scroll_stack: Vec::new(),
            projection_child_path: Vec::new(),
            view_delta: ViewDeltaScratch::with_capacity(DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY),
        }
    }
}
