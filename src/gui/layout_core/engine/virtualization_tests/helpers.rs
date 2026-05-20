use super::*;

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

pub(crate) fn scroll_with_content(
    content_kind: ContainerKind,
    total_children: u64,
    axis: VirtualizationAxis,
    overscan_px: f32,
) -> LayoutNode {
    let children = (0..total_children)
        .map(|index| SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 10.0)),
        })
        .collect::<Vec<_>>();
    let content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: content_kind,
            spacing: 1.0,
            wrap: WrapPolicy {
                item_gap: 2.0,
                line_gap: 2.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    );

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis,
                overscan_px,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: content,
        }],
    )
}

pub(crate) fn fixed_virtualized_scroll_root(row_height: f32) -> LayoutNode {
    let children = (0..128_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(row_height),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(180.0, 20.0)),
        })
        .collect::<Vec<_>>();

    LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 24.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 2.0,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        }],
    )
}
