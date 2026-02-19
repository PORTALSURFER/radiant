//! Deterministic two-pass layout engine for strict slot-based trees.

mod context;
mod helpers;
mod layout;
mod measure;
mod types;

use super::constraints::Constraints;
use super::tree::{LayoutNode, NodeId, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{BTreeSet, HashMap};

use context::LayoutContext;
pub use types::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutOutput, LayoutState, OverflowInfo,
};

/// Paint context consumed by widget paint implementations.
#[derive(Default)]
pub struct PaintContext;

/// Public widget layout contract.
pub trait Widget {
    /// Return the preferred size for the provided constraints.
    fn measure(&self, constraints: Constraints) -> Vector2;

    /// Paint the widget in the assigned rectangle.
    fn paint(&self, rect: Rect, ctx: &mut PaintContext);
}

/// Public container layout contract.
pub trait Container {
    /// Measure this container from its children and incoming constraints.
    fn measure(&self, children: &[SlotChild], constraints: Constraints) -> Vector2;

    /// Layout this container's children into `rect`.
    fn layout(&self, children: &[SlotChild], rect: Rect) -> Vec<(NodeId, Rect)>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ConstraintKey {
    min_w: u32,
    max_w: u32,
    min_h: u32,
    max_h: u32,
}

impl ConstraintKey {
    fn from_constraints(constraints: Constraints) -> Self {
        Self {
            min_w: constraints.min_w.to_bits(),
            max_w: constraints.max_w.to_bits(),
            min_h: constraints.min_h.to_bits(),
            max_h: constraints.max_h.to_bits(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct MeasureCacheKey {
    node_id: NodeId,
    constraints: ConstraintKey,
    state_version: u64,
}

impl MeasureCacheKey {
    fn new(node: &LayoutNode, constraints: Constraints) -> Self {
        Self {
            node_id: node.id(),
            constraints: ConstraintKey::from_constraints(constraints),
            state_version: node.state_version(),
        }
    }
}

#[derive(Default)]
pub struct LayoutEngine {
    measure_cache: HashMap<MeasureCacheKey, Vector2>,
    layout_dirty: BTreeSet<NodeId>,
    measure_dirty: BTreeSet<NodeId>,
}

impl LayoutEngine {
    /// Mark a node as geometry-dirty.
    pub fn mark_layout_dirty(&mut self, node_id: NodeId) {
        self.layout_dirty.insert(node_id);
    }

    /// Mark a node as intrinsic-measure dirty.
    pub fn mark_measure_dirty(&mut self, node_id: NodeId) {
        self.measure_dirty.insert(node_id);
    }

    /// Clear all dirty markers.
    pub fn clear_dirty(&mut self) {
        self.layout_dirty.clear();
        self.measure_dirty.clear();
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
