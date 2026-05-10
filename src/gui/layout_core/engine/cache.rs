//! Cache keys and virtualized linear metrics for the layout engine.

use super::super::constraints::Constraints;
use super::super::model::{
    CrossAlign, MainAlign, OverflowPolicy, SizeModeCross, SizeModeMain, VirtualizationAxis,
};
use super::super::tree::{ContainerNode, LayoutNode, NodeId};
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui::layout_core::engine) struct ConstraintKey {
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
pub(in crate::gui::layout_core::engine) struct MeasureCacheKey {
    pub(super) node_id: NodeId,
    constraints: ConstraintKey,
    state_version: u64,
}

impl MeasureCacheKey {
    pub(super) fn new(node: &LayoutNode, constraints: Constraints) -> Self {
        Self {
            node_id: node.id(),
            constraints: ConstraintKey::from_constraints(constraints),
            state_version: node.state_version(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui::layout_core::engine) struct VirtualizationCacheKey {
    node_id: NodeId,
    constraints: ConstraintKey,
    axis: VirtualizationAxis,
    child_count: usize,
    policy_fingerprint: u64,
}

impl VirtualizationCacheKey {
    pub(super) fn new(
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
pub(in crate::gui::layout_core::engine) struct VirtualSpan {
    pub(super) start: f32,
    pub(super) end: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::gui::layout_core::engine) struct LinearVirtualMetrics {
    pub(super) spans: Vec<VirtualSpan>,
    pub(super) main_sizes: Vec<f32>,
    pub(super) total_main: f32,
    pub(super) leading_offset: f32,
    pub(super) distributed_spacing: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::layout_core::engine) struct ResolvedLinearWindow {
    pub(super) first: usize,
    pub(super) last_exclusive: usize,
    pub(super) cursor_main_start: f32,
    pub(super) metrics: Arc<LinearVirtualMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui::layout_core::engine) struct CachedVirtualMetrics {
    pub(super) metrics: Arc<LinearVirtualMetrics>,
    pub(super) dependencies: BTreeSet<NodeId>,
}

pub(in crate::gui::layout_core::engine) fn virtualization_policy_fingerprint(
    container: &ContainerNode,
) -> u64 {
    fn push_f32(hasher: &mut std::collections::hash_map::DefaultHasher, value: f32) {
        value.to_bits().hash(hasher);
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
