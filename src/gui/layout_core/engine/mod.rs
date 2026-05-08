//! Deterministic two-pass layout engine for strict slot-based trees.

mod context;
mod dirty;
mod helpers;
mod layout;
mod measure;
mod types;

use super::constraints::Constraints;
use super::model::{
    CrossAlign, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain, VirtualizationAxis,
};
use super::tree::{ContainerNode, LayoutNode, NodeId};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{BTreeSet, HashMap};

use context::LayoutContext;
pub use types::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive, LayoutDiagnostic,
    LayoutDiagnosticCode, LayoutOutput, LayoutState, LayoutStats, OverflowInfo, VirtualWindowInfo,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct VirtualizationCacheKey {
    node_id: NodeId,
    constraints: ConstraintKey,
    axis: VirtualizationAxis,
    child_count: usize,
    policy_fingerprint: u64,
}

impl VirtualizationCacheKey {
    fn new(
        node_id: NodeId,
        constraints: Constraints,
        axis: VirtualizationAxis,
        child_count: usize,
        policy_fingerprint: u64,
    ) -> Self {
        Self {
            node_id,
            constraints: ConstraintKey::from_constraints(constraints),
            axis,
            child_count,
            policy_fingerprint,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct VirtualSpan {
    pub(super) start: f32,
    pub(super) end: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct LinearVirtualMetrics {
    pub(super) spans: Vec<VirtualSpan>,
    pub(super) main_sizes: Vec<f32>,
    pub(super) total_main: f32,
    pub(super) leading_offset: f32,
    pub(super) distributed_spacing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ResolvedLinearWindow {
    pub(super) first: usize,
    pub(super) last_exclusive: usize,
    pub(super) cursor_main_start: f32,
    pub(super) distributed_spacing: f32,
    pub(super) main_sizes: Vec<f32>,
    pub(super) leading_offset: f32,
    pub(super) total_main: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CachedVirtualMetrics {
    pub(super) metrics: LinearVirtualMetrics,
    pub(super) dependencies: BTreeSet<NodeId>,
}

/// Reusable stateful layout engine with measurement and virtualization caches.
#[derive(Default)]
pub struct LayoutEngine {
    measure_cache: HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    layout_dirty: BTreeSet<NodeId>,
    measure_dirty: BTreeSet<NodeId>,
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

    fn mark_subtree_dirty(&mut self, root: &LayoutNode, node_id: NodeId, measure: bool) {
        let mut marked = BTreeSet::new();
        let mut path = Vec::new();
        if !dirty::collect_path_and_descendants(root, node_id, &mut path, &mut marked) {
            marked.insert(node_id);
        }
        for id in marked {
            if measure {
                self.measure_dirty.insert(id);
            } else {
                self.layout_dirty.insert(id);
            }
            self.invalidate_virtual_cache_for(id);
        }
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

pub(super) fn virtualization_policy_fingerprint(container: &ContainerNode) -> u64 {
    fn push_f32(hasher: &mut std::collections::hash_map::DefaultHasher, value: f32) {
        std::hash::Hash::hash(&value.to_bits(), hasher);
    }

    fn main_mode_code(mode: SizeModeMain) -> u8 {
        match mode {
            SizeModeMain::Fixed(_) => 0,
            SizeModeMain::Fill(_) => 1,
            SizeModeMain::Percent(_) => 2,
            SizeModeMain::Intrinsic => 3,
        }
    }

    fn cross_mode_code(mode: SizeModeCross) -> u8 {
        match mode {
            SizeModeCross::Fixed(_) => 0,
            SizeModeCross::Fill => 1,
            SizeModeCross::Intrinsic => 2,
        }
    }

    fn align_main_code(value: MainAlign) -> u8 {
        match value {
            MainAlign::Start => 0,
            MainAlign::Center => 1,
            MainAlign::End => 2,
            MainAlign::SpaceBetween => 3,
            MainAlign::SpaceAround => 4,
            MainAlign::SpaceEvenly => 5,
        }
    }

    fn align_cross_code(value: CrossAlign) -> u8 {
        match value {
            CrossAlign::Start => 0,
            CrossAlign::Center => 1,
            CrossAlign::End => 2,
            CrossAlign::Stretch => 3,
        }
    }

    fn overflow_code(value: OverflowPolicy) -> u8 {
        match value {
            OverflowPolicy::Clip => 0,
            OverflowPolicy::Scroll => 1,
            OverflowPolicy::Wrap => 2,
            OverflowPolicy::Shrink => 3,
        }
    }

    use std::hash::Hasher;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u8(align_main_code(container.policy.align_main));
    hasher.write_u8(align_cross_code(container.policy.align_cross));
    hasher.write_u8(overflow_code(container.policy.overflow));
    push_f32(&mut hasher, container.policy.spacing);
    for child in &container.children {
        hasher.write_u64(child.child.id());
        hasher.write_u64(child.child.state_version());
        hasher.write_u8(main_mode_code(child.slot.size_main));
        match child.slot.size_main {
            SizeModeMain::Fixed(value)
            | SizeModeMain::Fill(value)
            | SizeModeMain::Percent(value) => push_f32(&mut hasher, value),
            SizeModeMain::Intrinsic => {}
        }
        hasher.write_u8(cross_mode_code(child.slot.size_cross));
        if let SizeModeCross::Fixed(value) = child.slot.size_cross {
            push_f32(&mut hasher, value);
        }
        push_f32(&mut hasher, child.slot.constraints.min_w);
        push_f32(&mut hasher, child.slot.constraints.max_w);
        push_f32(&mut hasher, child.slot.constraints.min_h);
        push_f32(&mut hasher, child.slot.constraints.max_h);
        push_f32(&mut hasher, child.slot.margin.left);
        push_f32(&mut hasher, child.slot.margin.right);
        push_f32(&mut hasher, child.slot.margin.top);
        push_f32(&mut hasher, child.slot.margin.bottom);
        hasher.write_u8(match child.slot.align_cross_override {
            None => 0,
            Some(value) => 1 + align_cross_code(value),
        });
        hasher.write_u8(u8::from(child.slot.allow_fixed_compress));
    }
    hasher.finish()
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
