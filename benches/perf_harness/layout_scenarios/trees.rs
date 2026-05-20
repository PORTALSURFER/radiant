//! Synthetic layout trees used by layout performance scenarios.

use radiant::layout::{
    ContainerKind, ContainerPolicy, LayoutNode, SizeModeCross, SizeModeMain, SlotChild, SlotParams,
    Vector2, VirtualizationAxis, VirtualizationPolicy,
};

pub(super) fn deep_nesting_tree() -> LayoutNode {
    let mut node = LayoutNode::widget(9_999, Vector2::new(8.0, 8.0));
    for id in (1..=300).rev() {
        node = LayoutNode::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::PaddingBox,
                padding: radiant::layout::Insets::all(1.0),
                ..ContainerPolicy::default()
            },
            vec![SlotChild::new(SlotParams::fill(), node)],
        );
    }
    node
}

pub(super) fn wrap_tree(count: u64) -> LayoutNode {
    let children = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(index + 2, Vector2::new(12.0, 8.0)),
            )
        })
        .collect();
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: radiant::layout::WrapPolicy {
                item_gap: 1.0,
                line_gap: 1.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    )
}

pub(super) fn virtualized_scroll_tree(count: u64, size_main: SizeModeMain) -> LayoutNode {
    let items = (0..count)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(120.0, 10.0)),
            )
        })
        .collect();

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: radiant::layout::OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 16.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 1.0,
                    ..ContainerPolicy::default()
                },
                items,
            ),
        )],
    )
}
