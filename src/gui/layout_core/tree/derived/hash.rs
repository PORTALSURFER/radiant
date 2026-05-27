//! Derived layout-tree fingerprint helpers.

use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, CrossAlign, MainAlign, OverflowPolicy, SizeModeCross,
    SizeModeMain, SlotParams, VirtualizationAxis,
};
use std::hash::{Hash, Hasher};

pub(super) fn policy_hash(policy: &ContainerPolicy, hasher: &mut impl Hasher) {
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
    hash_f32(policy.floating.offset.x, hasher);
    hash_f32(policy.floating.offset.y, hasher);
    hash_f32(policy.floating.size.x, hasher);
    hash_f32(policy.floating.size.y, hasher);
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

pub(super) fn slot_hash(slot: &SlotParams, hasher: &mut impl Hasher) {
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
        ContainerKind::FloatingLayer => 10,
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
