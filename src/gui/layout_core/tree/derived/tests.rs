use super::*;
use crate::{
    gui::layout_core::{
        constraints::Constraints,
        model::{Insets, SizeModeCross, SizeModeMain, SlotParams},
    },
    gui::types::Vector2,
};

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
                crate::gui::layout_core::tree::LayoutNode::widget(
                    index + 10,
                    Vector2::new(8.0, 8.0),
                ),
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
                crate::gui::layout_core::tree::LayoutNode::widget(
                    index + 10,
                    Vector2::new(8.0, 8.0),
                ),
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
                    margin: Insets {
                        top: -2.0,
                        bottom: 2.0,
                        ..Default::default()
                    },
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                crate::gui::layout_core::tree::LayoutNode::widget(
                    index + 10,
                    Vector2::new(8.0, 8.0),
                ),
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

#[test]
fn widget_state_version_tracks_intrinsic_size() {
    let compact = crate::gui::layout_core::tree::LayoutNode::widget(10, Vector2::new(80.0, 20.0));
    let wide = crate::gui::layout_core::tree::LayoutNode::widget(10, Vector2::new(160.0, 20.0));

    assert_ne!(compact.state_version(), wide.state_version());
}
