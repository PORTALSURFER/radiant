use super::super::super::tree::NodeId;
use super::{LayoutDebugPrimitive, LayoutDiagnostic, LayoutStats, OverflowInfo, VirtualWindowInfo};
use crate::gui::types::Rect;
use std::collections::{BTreeMap, BTreeSet};

/// Final layout output from a full measure/layout pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutOutput {
    /// Final rounded rectangles by node id.
    pub rects: BTreeMap<NodeId, Rect>,
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
    /// Virtualization window metadata keyed by scroll container id.
    pub virtual_windows: BTreeMap<NodeId, VirtualWindowInfo>,
    /// Traversal counters collected during this layout pass.
    pub stats: LayoutStats,
}

impl LayoutOutput {
    pub(in crate::gui::layout_core) fn clear_reusing_storage(&mut self) {
        self.rects.clear();
        self.overflowed.clear();
        self.overflow_flags.clear();
        self.diagnostics.clear();
        self.debug_primitives.clear();
        self.viewport_bounds.clear();
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
}
