//! Deterministic two-pass layout engine for strict slot-based trees.

mod cache;
mod context;
mod direct;
mod dirty;
mod helpers;
mod layout;
mod measure;
mod types;

use super::constraints::Constraints;
use super::tree::{LayoutNode, NodeId};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{HashMap, HashSet};

use cache::{CachedVirtualMetrics, MeasureCacheKey, VirtualizationCacheKey};
use context::{LayoutContext, LayoutScratch};
pub use types::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutOutput, LayoutState, LayoutStats, OverflowInfo, VirtualWindowInfo,
};

/// Reusable stateful layout engine with measurement and virtualization caches.
#[derive(Default)]
pub struct LayoutEngine {
    measure_cache: HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    scratch: LayoutScratch,
    layout_dirty: HashSet<NodeId>,
    measure_dirty: HashSet<NodeId>,
}

impl LayoutEngine {
    /// Mark a node as geometry-dirty.
    pub fn mark_layout_dirty(&mut self, node_id: NodeId) {
        self.layout_dirty.insert(node_id);
        self.invalidate_virtual_cache_for(node_id);
    }

    /// Mark a node as intrinsic-measure dirty.
    pub fn mark_measure_dirty(&mut self, node_id: NodeId) {
        self.measure_dirty.insert(node_id);
        self.invalidate_virtual_cache_for(node_id);
    }

    /// Mark a node subtree as geometry-dirty, including ancestor path nodes.
    pub fn mark_layout_dirty_subtree(&mut self, root: &LayoutNode, node_id: NodeId) {
        self.mark_subtree_dirty(root, node_id, false);
    }

    /// Mark a node subtree as measure-dirty, including ancestor path nodes.
    pub fn mark_measure_dirty_subtree(&mut self, root: &LayoutNode, node_id: NodeId) {
        self.mark_subtree_dirty(root, node_id, true);
    }

    /// Clear all dirty markers.
    pub fn clear_dirty(&mut self) {
        self.layout_dirty.clear();
        self.measure_dirty.clear();
    }

    fn invalidate_virtual_cache_for(&mut self, node_id: NodeId) {
        self.virtual_cache
            .retain(|_, entry| !entry.dependencies.contains(&node_id));
    }

    fn invalidate_virtual_cache_for_any(&mut self, node_ids: &HashSet<NodeId>) {
        if node_ids.is_empty() {
            return;
        }
        self.virtual_cache
            .retain(|_, entry| entry.dependencies.is_disjoint(node_ids));
    }

    fn mark_subtree_dirty(&mut self, root: &LayoutNode, node_id: NodeId, measure: bool) {
        let mut marked = HashSet::new();
        let mut path = Vec::new();
        if !dirty::collect_path_and_descendants(root, node_id, &mut path, &mut marked) {
            marked.insert(node_id);
        }
        for id in &marked {
            if measure {
                self.measure_dirty.insert(*id);
            } else {
                self.layout_dirty.insert(*id);
            }
        }
        self.invalidate_virtual_cache_for_any(&marked);
    }

    /// Compute layout output for `root` in `root_rect` using default state/options.
    pub fn layout(&mut self, root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
        self.layout_with_state(
            root,
            root_rect,
            &LayoutState::default(),
            LayoutDebugOptions::default(),
        )
    }

    /// Compute layout output with dynamic layout state and debug output controls.
    pub fn layout_with_state(
        &mut self,
        root: &LayoutNode,
        root_rect: Rect,
        state: &LayoutState,
        debug: LayoutDebugOptions,
    ) -> LayoutOutput {
        let constraints = Constraints {
            min_w: 0.0,
            max_w: root_rect.width().max(0.0),
            min_h: 0.0,
            max_h: root_rect.height().max(0.0),
        };

        let output = {
            let debug_node_filter = if debug.enabled && !self.layout_dirty.is_empty() {
                Some(&self.layout_dirty)
            } else {
                None
            };
            let mut context = LayoutContext::new(
                &mut self.measure_cache,
                &mut self.virtual_cache,
                &mut self.scratch,
                &self.measure_dirty,
                state,
                debug,
                debug_node_filter,
            );
            let normalized = context.normalize_constraints(root.id(), constraints);
            measure::measure_node(root, normalized, &mut context);
            layout::layout_node(root, round_rect(root_rect), &mut context);
            context.output
        };

        self.clear_dirty();
        output
    }
}

/// Measure and layout a strict slot tree into rounded rectangles.
pub fn layout_tree(root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    engine.layout(root, root_rect)
}

/// Measure and layout a strict slot tree with stateful container input.
///
/// This is the single-call entry point for callers that want scroll offsets or
/// debug primitives without manually reusing a [`LayoutEngine`].
pub fn layout_tree_with_state(
    root: &LayoutNode,
    root_rect: Rect,
    state: &LayoutState,
    debug: LayoutDebugOptions,
) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    engine.layout_with_state(root, root_rect, state, debug)
}

pub(super) fn round_rect(rect: Rect) -> Rect {
    let min_x = rect.min.x.floor();
    let min_y = rect.min.y.floor();
    let width = rect.width().round().max(0.0);
    let height = rect.height().round().max(0.0);
    Rect::from_min_size(Point::new(min_x, min_y), Vector2::new(width, height))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;

#[cfg(test)]
mod stress_tests;

#[cfg(test)]
mod virtualization_tests;

#[cfg(test)]
mod contract_tests;
