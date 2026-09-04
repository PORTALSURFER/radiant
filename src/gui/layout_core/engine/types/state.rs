use super::super::super::tree::NodeId;
use crate::gui::types::Vector2;
use std::collections::BTreeMap;

/// Runtime-owned generation fences for one mounted scroll container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScrollRuntimeState {
    /// Mount incarnation that owns this state.
    pub mount_generation: u64,
    /// Last controlled offset generation accepted.
    pub controlled_generation: Option<u64>,
    /// Last reveal request generation consumed, including valid no-ops.
    pub request_generation: Option<u64>,
    /// Whether the initial offset has been seeded for this mount.
    pub initial_seeded: bool,
}

/// Dynamic layout state supplied by callers for stateful containers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutState {
    /// Per-node scroll offsets used by `ScrollView` containers.
    pub scroll_offsets: BTreeMap<NodeId, Vector2>,
    /// Generation-fenced declarative scroll state keyed by mounted node id.
    pub scroll_runtime: BTreeMap<NodeId, ScrollRuntimeState>,
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
