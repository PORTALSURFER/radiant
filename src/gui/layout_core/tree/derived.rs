//! Derived layout-tree cache state and precomputed linear metrics.

use super::{LayoutNode, NodeId, SlotChild};
use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, CrossAlign, MainAlign, OverflowPolicy, SizeModeCross,
    SizeModeMain, SlotParams, VirtualizationAxis,
};
use std::hash::{Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ContainerDerivedState {
    pub(super) state_version: u64,
    pub(super) horizontal_metrics: KnownMainMetrics,
    pub(super) vertical_metrics: KnownMainMetrics,
}

pub(super) fn container_derived_state(
    id: NodeId,
    policy: &ContainerPolicy,
    children: &[SlotChild],
) -> ContainerDerivedState {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    policy_hash(policy, &mut hasher);
    children.len().hash(&mut hasher);
    for child in children {
        child.child.id().hash(&mut hasher);
        child.child.state_version().hash(&mut hasher);
        slot_hash(&child.slot, &mut hasher);
    }
    let horizontal_metrics = if policy.kind == ContainerKind::Row {
        known_main_metrics(true, policy.spacing, children)
    } else {
        KnownMainMetrics::default()
    };
    let vertical_metrics = if policy.kind == ContainerKind::Column {
        known_main_metrics(false, policy.spacing, children)
    } else {
        KnownMainMetrics::default()
    };
    ContainerDerivedState {
        state_version: hasher.finish(),
        horizontal_metrics,
        vertical_metrics,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct KnownMainMetrics {
    pub(super) extent: Option<f32>,
    pub(super) uniform_main: Option<f32>,
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

fn known_main_metrics(horizontal: bool, spacing: f32, children: &[SlotChild]) -> KnownMainMetrics {
    let spacing_total = spacing.max(0.0) * children.len().saturating_sub(1) as f32;
    let mut total = spacing_total;
    let mut uniform_main: Option<f32> = None;
    let mut uniform_valid = true;
    for child in children {
        let has_main_margin = has_main_axis_margin(horizontal, child);
        let size = match child.slot.size_main {
            SizeModeMain::Fixed(size) => size.max(0.0),
            SizeModeMain::Intrinsic => match direct_widget_intrinsic_main(horizontal, &child.child)
            {
                Some(size) => size,
                None => return KnownMainMetrics::default(),
            },
            SizeModeMain::Fill(_) | SizeModeMain::Percent(_) => return KnownMainMetrics::default(),
        };
        let constraints = child.slot.constraints.normalized();
        let main = if horizontal {
            constraints.clamp_w(size) + child.slot.margin.left + child.slot.margin.right
        } else {
            constraints.clamp_h(size) + child.slot.margin.top + child.slot.margin.bottom
        };
        let item_main = if horizontal {
            constraints.clamp_w(size)
        } else {
            constraints.clamp_h(size)
        };
        if has_main_margin {
            uniform_valid = false;
        } else if let Some(expected) = uniform_main {
            if (expected - item_main).abs() > f32::EPSILON {
                uniform_valid = false;
            }
        } else if uniform_main.is_none() {
            uniform_main = Some(item_main);
        }
        total += main;
    }
    KnownMainMetrics {
        extent: Some(total),
        uniform_main: uniform_valid.then_some(uniform_main).flatten(),
    }
}

fn direct_widget_intrinsic_main(horizontal: bool, node: &LayoutNode) -> Option<f32> {
    let LayoutNode::Widget(widget) = node else {
        return None;
    };
    let main = if horizontal {
        widget.intrinsic.x
    } else {
        widget.intrinsic.y
    };
    main.is_finite().then_some(main.max(0.0))
}

fn has_main_axis_margin(horizontal: bool, child: &SlotChild) -> bool {
    let (before, after) = if horizontal {
        (child.slot.margin.left, child.slot.margin.right)
    } else {
        (child.slot.margin.top, child.slot.margin.bottom)
    };
    before.abs() > f32::EPSILON || after.abs() > f32::EPSILON
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{gui::layout_core::constraints::Constraints, gui::types::Vector2};

    #[test]
    fn container_precomputes_uniform_main_size_with_extent() {
        let children = (0..4_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(24.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    LayoutNode::widget(index + 10, Vector2::new(8.0, 8.0)),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_vertical, Some(24.0));
        assert_eq!(
            container.known_main_extent_vertical,
            Some(24.0 * 4.0 + 2.0 * 3.0)
        );
        assert_eq!(container.known_main_extent_horizontal, None);
    }

    #[test]
    fn container_precomputes_only_matching_linear_axis() {
        let children = (0..3_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(18.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: Default::default(),
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    LayoutNode::widget(index + 10, Vector2::new(8.0, 8.0)),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 3.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_horizontal, Some(18.0));
        assert_eq!(
            container.known_main_extent_horizontal,
            Some(18.0 * 3.0 + 3.0 * 2.0)
        );
        assert_eq!(container.known_main_extent_vertical, None);
    }

    #[test]
    fn container_does_not_mark_margin_rows_as_uniform() {
        let children = (0..4_u64)
            .map(|index| {
                SlotChild::new(
                    SlotParams {
                        size_main: SizeModeMain::Fixed(24.0),
                        size_cross: SizeModeCross::Fill,
                        constraints: Constraints::unconstrained(),
                        margin: crate::gui::layout_core::model::Insets {
                            top: -2.0,
                            bottom: 2.0,
                            ..Default::default()
                        },
                        align_cross_override: None,
                        allow_fixed_compress: false,
                    },
                    LayoutNode::widget(index + 10, Vector2::new(8.0, 8.0)),
                )
            })
            .collect();
        let container = crate::layout::ContainerNode::new(
            1,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            children,
        );

        assert_eq!(container.known_uniform_main_vertical, None);
        assert_eq!(
            container.known_main_extent_vertical,
            Some(24.0 * 4.0 + 2.0 * 3.0)
        );
    }
}
