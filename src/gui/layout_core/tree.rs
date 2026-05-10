//! Layout tree node definitions for the slot-based engine.

use super::model::{
    ContainerKind, ContainerPolicy, CrossAlign, MainAlign, OverflowPolicy, SizeModeCross,
    SizeModeMain, SlotParams, VirtualizationAxis,
};
use crate::gui::types::Vector2;
use std::hash::{Hash, Hasher};

/// Stable node identifier for layout cache keys and output maps.
pub type NodeId = u64;

/// A child attachment entry owned by a parent container slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotChild {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: LayoutNode,
}

impl SlotChild {
    /// Build a parent-owned slot attachment.
    pub fn new(slot: SlotParams, child: LayoutNode) -> Self {
        Self { slot, child }
    }
}

/// A container node with deterministic layout policy and slot children.
#[derive(Clone, Debug, PartialEq)]
pub struct ContainerNode {
    /// Stable node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Ordered slot children.
    pub children: Vec<SlotChild>,
    /// Version used by persistent layout caches.
    pub(crate) state_version: u64,
    /// Precomputed horizontal fixed-row/column extent when every child has a fixed main size.
    pub(crate) fixed_main_extent_horizontal: Option<f32>,
    /// Precomputed vertical fixed-row/column extent when every child has a fixed main size.
    pub(crate) fixed_main_extent_vertical: Option<f32>,
}

impl ContainerNode {
    /// Construct a container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        let state_version = container_state_version(id, &policy, &children);
        let fixed_main_extent_horizontal = fixed_main_extent(true, policy.spacing, &children);
        let fixed_main_extent_vertical = fixed_main_extent(false, policy.spacing, &children);
        Self {
            id,
            policy,
            children,
            state_version,
            fixed_main_extent_horizontal,
            fixed_main_extent_vertical,
        }
    }
}

/// A widget node with intrinsic size hints.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetNode {
    /// Stable node id.
    pub id: NodeId,
    /// Intrinsic preferred size in logical pixels.
    pub intrinsic: Vector2,
    /// Version used by persistent layout caches.
    pub state_version: u64,
}

impl WidgetNode {
    /// Construct a widget node with an intrinsic size hint.
    pub fn new(id: NodeId, intrinsic: Vector2) -> Self {
        Self {
            id,
            intrinsic,
            state_version: 0,
        }
    }
}

/// A layout node in the strict slot-based tree.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutNode {
    /// A container that owns slots and lays out child nodes.
    Container(ContainerNode),
    /// A widget leaf that contributes intrinsic sizing information.
    Widget(WidgetNode),
}

impl LayoutNode {
    /// Return this node's stable id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Container(node) => node.id,
            Self::Widget(node) => node.id,
        }
    }

    /// Return this node's cache state version.
    pub fn state_version(&self) -> u64 {
        match self {
            Self::Container(node) => node.state_version,
            Self::Widget(node) => node.state_version,
        }
    }

    /// Convenience constructor for a leaf widget node.
    pub fn widget(id: NodeId, intrinsic: Vector2) -> Self {
        Self::Widget(WidgetNode::new(id, intrinsic))
    }

    /// Convenience constructor for a container node.
    pub fn container(id: NodeId, policy: ContainerPolicy, children: Vec<SlotChild>) -> Self {
        Self::Container(ContainerNode::new(id, policy, children))
    }
}

fn container_state_version(id: NodeId, policy: &ContainerPolicy, children: &[SlotChild]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    policy_hash(policy, &mut hasher);
    children.len().hash(&mut hasher);
    for child in children {
        child.child.id().hash(&mut hasher);
        child.child.state_version().hash(&mut hasher);
        slot_hash(&child.slot, &mut hasher);
    }
    hasher.finish()
}

fn policy_hash(policy: &ContainerPolicy, hasher: &mut impl Hasher) {
    container_kind_code(policy.kind).hash(hasher);
    hash_f32(policy.spacing, hasher);
    hash_f32(policy.padding.left, hasher);
    hash_f32(policy.padding.right, hasher);
    hash_f32(policy.padding.top, hasher);
    hash_f32(policy.padding.bottom, hasher);
    main_align_code(policy.align_main).hash(hasher);
    cross_align_code(policy.align_cross).hash(hasher);
    overflow_code(policy.overflow).hash(hasher);
    policy.grid.columns.hash(hasher);
    hash_f32(policy.grid.column_gap, hasher);
    hash_f32(policy.grid.row_gap, hasher);
    hash_f32(policy.wrap.item_gap, hasher);
    hash_f32(policy.wrap.line_gap, hasher);
    policy.aspect_ratio.map(f32::to_bits).hash(hasher);
    for breakpoint in &policy.switch_breakpoints {
        hash_f32(breakpoint.min_width, hasher);
        hash_f32(breakpoint.max_width, hasher);
    }
    policy.virtualization.is_some().hash(hasher);
    if let Some(virtualization) = policy.virtualization {
        virtualization.enabled.hash(hasher);
        virtualization_axis_code(virtualization.axis).hash(hasher);
        hash_f32(virtualization.overscan_px, hasher);
    }
}

fn slot_hash(slot: &SlotParams, hasher: &mut impl Hasher) {
    size_mode_main_hash(slot.size_main, hasher);
    size_mode_cross_hash(slot.size_cross, hasher);
    hash_f32(slot.constraints.min_w, hasher);
    hash_f32(slot.constraints.max_w, hasher);
    hash_f32(slot.constraints.min_h, hasher);
    hash_f32(slot.constraints.max_h, hasher);
    hash_f32(slot.margin.left, hasher);
    hash_f32(slot.margin.right, hasher);
    hash_f32(slot.margin.top, hasher);
    hash_f32(slot.margin.bottom, hasher);
    slot.align_cross_override.map(cross_align_code).hash(hasher);
    slot.allow_fixed_compress.hash(hasher);
}

fn hash_f32(value: f32, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

fn fixed_main_extent(horizontal: bool, spacing: f32, children: &[SlotChild]) -> Option<f32> {
    let spacing_total = spacing.max(0.0) * children.len().saturating_sub(1) as f32;
    let mut total = spacing_total;
    for child in children {
        let SizeModeMain::Fixed(size) = child.slot.size_main else {
            return None;
        };
        let constraints = child.slot.constraints.normalized();
        let main = if horizontal {
            constraints.clamp_w(size.max(0.0)) + child.slot.margin.left + child.slot.margin.right
        } else {
            constraints.clamp_h(size.max(0.0)) + child.slot.margin.top + child.slot.margin.bottom
        };
        total += main;
    }
    Some(total)
}

fn container_kind_code(value: ContainerKind) -> u8 {
    match value {
        ContainerKind::Row => 0,
        ContainerKind::Column => 1,
        ContainerKind::Stack => 2,
        ContainerKind::PaddingBox => 3,
        ContainerKind::AlignBox => 4,
        ContainerKind::AspectBox => 5,
        ContainerKind::Grid => 6,
        ContainerKind::ScrollView => 7,
        ContainerKind::Wrap => 8,
        ContainerKind::SwitchLayout => 9,
    }
}

fn main_align_code(value: MainAlign) -> u8 {
    match value {
        MainAlign::Start => 0,
        MainAlign::Center => 1,
        MainAlign::End => 2,
        MainAlign::SpaceBetween => 3,
        MainAlign::SpaceAround => 4,
        MainAlign::SpaceEvenly => 5,
    }
}

fn cross_align_code(value: CrossAlign) -> u8 {
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

fn virtualization_axis_code(value: VirtualizationAxis) -> u8 {
    match value {
        VirtualizationAxis::Vertical => 0,
        VirtualizationAxis::Horizontal => 1,
    }
}

fn size_mode_main_hash(value: SizeModeMain, hasher: &mut impl Hasher) {
    match value {
        SizeModeMain::Fixed(size) => {
            0_u8.hash(hasher);
            hash_f32(size, hasher);
        }
        SizeModeMain::Fill(weight) => {
            1_u8.hash(hasher);
            hash_f32(weight, hasher);
        }
        SizeModeMain::Percent(percent) => {
            2_u8.hash(hasher);
            hash_f32(percent, hasher);
        }
        SizeModeMain::Intrinsic => 3_u8.hash(hasher),
    }
}

fn size_mode_cross_hash(value: SizeModeCross, hasher: &mut impl Hasher) {
    match value {
        SizeModeCross::Fixed(size) => {
            0_u8.hash(hasher);
            hash_f32(size, hasher);
        }
        SizeModeCross::Fill => 1_u8.hash(hasher),
        SizeModeCross::Intrinsic => 2_u8.hash(hasher),
    }
}
