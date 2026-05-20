use crate::gui::{
    layout_core::{
        constraints::Constraints,
        model::{
            ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross, SizeModeMain,
            SlotParams, VirtualizationAxis, VirtualizationPolicy,
        },
        tree::{LayoutNode, SlotChild},
    },
    types::Vector2,
};

pub(super) fn fixed_virtualized_root(child_count: u64, row_height: f32) -> LayoutNode {
    let children = (0..child_count)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(row_height),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, row_height)),
            )
        })
        .collect();
    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    )
}
