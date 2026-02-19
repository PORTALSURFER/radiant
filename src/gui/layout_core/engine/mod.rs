//! Deterministic two-pass layout engine for strict slot-based trees.

mod helpers;
mod layout;
mod measure;

use super::constraints::Constraints;
use super::model::OverflowPolicy;
use super::tree::{LayoutNode, NodeId, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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

/// Layout diagnostic emitted when invalid states are normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutDiagnostic {
    /// Node that triggered the diagnostic.
    pub node_id: NodeId,
    /// Human-readable diagnostic message.
    pub message: String,
}

/// Overflow metadata recorded for one node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverflowInfo {
    /// True when width overflowed.
    pub x: bool,
    /// True when height overflowed.
    pub y: bool,
    /// Policy used when handling overflow.
    pub policy: OverflowPolicy,
}

impl Default for OverflowInfo {
    fn default() -> Self {
        Self {
            x: false,
            y: false,
            policy: OverflowPolicy::Clip,
        }
    }
}

/// Final layout output from `layout_tree`.
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

    /// Compute layout output for `root` in `root_rect`.
    pub fn layout(&mut self, root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
        let constraints = Constraints::new(
            0.0,
            root_rect.width().max(0.0),
            0.0,
            root_rect.height().max(0.0),
        );
        let output = {
            let mut context = LayoutContext::new(&mut self.measure_cache, &self.measure_dirty);
            measure::measure_node(root, constraints, &mut context);
            layout::layout_node(root, round_rect(root_rect), &mut context);
            context.output
        };
        self.clear_dirty();
        output
    }
}

pub(super) struct LayoutContext<'a> {
    measured: HashMap<MeasureCacheKey, Vector2>,
    cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    measure_dirty: &'a BTreeSet<NodeId>,
    output: LayoutOutput,
}

impl<'a> LayoutContext<'a> {
    fn new(
        cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
        measure_dirty: &'a BTreeSet<NodeId>,
    ) -> Self {
        Self {
            measured: HashMap::new(),
            cache,
            measure_dirty,
            output: LayoutOutput::default(),
        }
    }

    pub(super) fn cached_measure(&self, key: MeasureCacheKey, node_id: NodeId) -> Option<Vector2> {
        if self.measure_dirty.contains(&node_id) {
            return None;
        }
        self.measured
            .get(&key)
            .copied()
            .or_else(|| self.cache.get(&key).copied())
    }

    pub(super) fn remember_measure(&mut self, key: MeasureCacheKey, value: Vector2) {
        self.measured.insert(key, value);
        self.cache.insert(key, value);
    }

    pub(super) fn record_overflow(
        &mut self,
        node_id: NodeId,
        policy: OverflowPolicy,
        x: bool,
        y: bool,
    ) {
        self.output.overflowed.insert(node_id);
        self.output
            .overflow_flags
            .insert(node_id, OverflowInfo { x, y, policy });
    }

    pub(super) fn push_diagnostic(&mut self, node_id: NodeId, message: impl Into<String>) {
        self.output.diagnostics.push(LayoutDiagnostic {
            node_id,
            message: message.into(),
        });
    }
}

/// Measure and layout a strict slot tree into rounded rectangles.
pub fn layout_tree(root: &LayoutNode, root_rect: Rect) -> LayoutOutput {
    let mut engine = LayoutEngine::default();
    engine.layout(root, root_rect)
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
