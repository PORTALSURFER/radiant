//! Property-style tests for core layout invariants.

use super::{LayoutDebugOptions, LayoutState, layout_tree, layout_tree_with_state};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, OverflowPolicy, SizeModeCross, SizeModeMain, SlotParams,
    VirtualizationAxis, VirtualizationPolicy,
};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

#[test]
fn layout_rects_are_finite_and_non_negative() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 3.0,
            overflow: OverflowPolicy::Clip,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(64.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(10.0, 128.0, 0.0, f32::INFINITY),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: true,
                },
                child: LayoutNode::widget(2, Vector2::new(64.0, 20.0)),
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(3, Vector2::new(80.0, 20.0)),
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(4, Vector2::new(100.0, 20.0)),
            },
        ],
    );

    for viewport in [
        Vector2::new(40.0, 24.0),
        Vector2::new(120.0, 40.0),
        Vector2::new(480.0, 80.0),
    ] {
        let output = layout_tree(&root, Rect::from_min_size(Point::new(0.0, 0.0), viewport));
        for rect in output.rects.values() {
            assert!(rect.min.x.is_finite());
            assert!(rect.min.y.is_finite());
            assert!(rect.width().is_finite());
            assert!(rect.height().is_finite());
            assert!(rect.width() >= 0.0);
            assert!(rect.height() >= 0.0);
        }
    }
}

#[test]
fn stateful_layout_is_deterministic() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(2, Vector2::new(600.0, 400.0)),
        }],
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(22.0, 33.0));

    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0));
    let first = layout_tree_with_state(&root, rect, &state, LayoutDebugOptions::default());
    let second = layout_tree_with_state(&root, rect, &state, LayoutDebugOptions::default());
    assert_eq!(first.rects, second.rects);
    assert_eq!(first.overflow_flags, second.overflow_flags);
}

#[test]
fn virtualized_layout_is_deterministic() {
    let items = (0..2_000_u64)
        .map(|index| SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Intrinsic,
                size_cross: SizeModeCross::Fill,
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(index + 10, Vector2::new(80.0, 10.0)),
        })
        .collect::<Vec<_>>();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 12.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing: 1.0,
                    ..ContainerPolicy::default()
                },
                items,
            ),
        }],
    );
    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(0.0, 900.0));
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0));
    let first = layout_tree_with_state(&root, rect, &state, LayoutDebugOptions::default());
    let second = layout_tree_with_state(&root, rect, &state, LayoutDebugOptions::default());
    assert_eq!(first.rects, second.rects);
    assert_eq!(first.virtual_windows, second.virtual_windows);
    assert_eq!(first.stats, second.stats);
}
