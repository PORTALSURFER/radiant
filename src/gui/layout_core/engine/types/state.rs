use super::super::super::tree::NodeId;
use crate::gui::types::Vector2;
use std::collections::BTreeMap;

/// Dynamic layout state supplied by callers for stateful containers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutState {
    /// Per-node scroll offsets used by `ScrollView` containers.
    pub scroll_offsets: BTreeMap<NodeId, Vector2>,
}

impl LayoutState {
    /// Return the configured scroll offset for a node or `(0, 0)` when absent.
    pub fn scroll_offset(&self, node_id: NodeId) -> Vector2 {
        self.scroll_offsets
            .get(&node_id)
            .copied()
            .unwrap_or_default()
    }
}
