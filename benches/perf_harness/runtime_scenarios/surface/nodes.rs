//! Synthetic surface trees used by runtime surface performance scenarios.

use radiant::{
    layout::{ContainerKind, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams, Vector2},
    runtime::{SurfaceChild, SurfaceNode},
    widgets::{TextWidget, WidgetSizing},
};

pub(super) fn runtime_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Intrinsic,
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    10_000 + index,
                    format!("Row {index}"),
                    WidgetSizing::fixed(Vector2::new(180.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        1,
        SurfaceNode::container(
            2,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

pub(super) fn text_paint_surface_node(count: u64) -> SurfaceNode<()> {
    let rows = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(22.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    40_000 + index,
                    format!(
                        "Track {index:04}  position {index:04}.{:02}  cached text row",
                        index % 97
                    ),
                    WidgetSizing::fixed(Vector2::new(520.0, 22.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        30_000,
        SurfaceNode::container(
            30_001,
            ContainerPolicy {
                kind: ContainerKind::Column,
                spacing: 1.0,
                ..ContainerPolicy::default()
            },
            rows,
        ),
    )
}

pub(super) fn horizontal_scroll_surface_node(count: u64) -> SurfaceNode<()> {
    let items = (0..count)
        .map(|index| {
            SurfaceChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(88.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: radiant::layout::Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                SurfaceNode::static_widget(TextWidget::new(
                    90_000 + index,
                    format!("Clip {index:04}"),
                    WidgetSizing::fixed(Vector2::new(88.0, 24.0)),
                )),
            )
        })
        .collect();
    SurfaceNode::scroll_area(
        80_000,
        SurfaceNode::container(
            80_001,
            ContainerPolicy {
                kind: ContainerKind::Row,
                spacing: 2.0,
                ..ContainerPolicy::default()
            },
            items,
        ),
    )
}
