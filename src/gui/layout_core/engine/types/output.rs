use super::super::super::tree::NodeId;
use super::{LayoutDebugPrimitive, LayoutDiagnostic, LayoutStats, OverflowInfo, VirtualWindowInfo};
use crate::gui::types::Rect;
use std::collections::{BTreeMap, BTreeSet};

/// Final layout output from a full measure/layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOutput {
    /// Final rounded rectangles by node id.
    pub rects: BTreeMap<NodeId, Rect>,
    omitted_nodes: BTreeSet<NodeId>,
    /// Node ids that overflowed available space.
    pub overflowed: BTreeSet<NodeId>,
    /// Per-node overflow metadata.
    pub overflow_flags: BTreeMap<NodeId, OverflowInfo>,
    /// Diagnostics collected during measure/layout normalization.
    pub diagnostics: Vec<LayoutDiagnostic>,
    /// Optional debug primitives emitted by the traversal.
    pub debug_primitives: Vec<LayoutDebugPrimitive>,
    /// Scroll viewport bounds keyed by scroll container id.
    pub viewport_bounds: BTreeMap<NodeId, Rect>,
    /// Scrollbar placement keyed by scroll container id. This keeps paint and
    /// hit testing on the same committed geometry as layout.
    pub scrollbar_placements: BTreeMap<NodeId, crate::gui::layout_core::ScrollbarPlacement>,
    /// Virtualization window metadata keyed by scroll container id.
    pub virtual_windows: BTreeMap<NodeId, VirtualWindowInfo>,
    /// Traversal counters collected during this layout pass.
    pub stats: LayoutStats,
}

impl LayoutOutput {
    pub(in crate::gui::layout_core) fn clear_reusing_storage(&mut self) {
        self.rects.clear();
        self.omitted_nodes.clear();
        self.overflowed.clear();
        self.overflow_flags.clear();
        self.diagnostics.clear();
        self.debug_primitives.clear();
        self.viewport_bounds.clear();
        self.scrollbar_placements.clear();
        self.virtual_windows.clear();
        self.stats = LayoutStats::default();
    }

    /// Return one resolved node rectangle or the caller-provided fallback.
    pub fn rect_for(&self, node_id: NodeId, fallback: Rect) -> Rect {
        self.rects.get(&node_id).copied().unwrap_or(fallback)
    }

    /// Return one resolved node rectangle clamped inside `bounds`.
    pub fn rect_for_clamped(&self, node_id: NodeId, fallback: Rect, bounds: Rect) -> Rect {
        self.rect_for(node_id, fallback).clamp_to(bounds)
    }

    pub(crate) fn record_omitted_node(&mut self, node_id: NodeId) {
        self.omitted_nodes.insert(node_id);
    }

    pub(crate) fn is_omitted(&self, node_id: NodeId) -> bool {
        self.omitted_nodes.contains(&node_id)
    }
}
