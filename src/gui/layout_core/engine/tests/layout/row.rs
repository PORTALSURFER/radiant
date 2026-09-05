use super::super::super::layout_tree;
use crate::gui::layout_core::WritingDirection;
use crate::gui::{
    layout_core::{
        constraints::Constraints,
        model::{ContainerKind, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn layout_tree_is_deterministic_for_same_input() {
    let child_a = LayoutNode::widget(2, Vector2::new(32.0, 20.0));
    let child_b = LayoutNode::widget(3, Vector2::new(64.0, 20.0));
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 8.0,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: child_a,
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: child_b,
            },
        ],
    );
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(400.0, 80.0));
    let first = layout_tree(&root, rect);
    let second = layout_tree(&root, rect);
    assert_eq!(first.rects, second.rects);
    assert_eq!(first.overflowed, second.overflowed);
}

#[test]
fn fill_children_compress_before_fixed_children() {
    let fill_a = LayoutNode::widget(2, Vector2::new(200.0, 20.0));
    let fixed = LayoutNode::widget(3, Vector2::new(80.0, 20.0));
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: fill_a,
            },
            SlotChild {
                slot: SlotParams {
                    size_main: SizeModeMain::Fixed(80.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(80.0, 80.0, 0.0, f32::INFINITY),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                child: fixed,
            },
        ],
    );
    let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0));
    let output = layout_tree(&root, rect);
    let fixed_rect = output.rects.get(&3).expect("fixed rect");
    assert!((fixed_rect.width() - 80.0).abs() < 0.5);
}

#[test]
fn fill_children_redistribute_after_constrained_child_clamps() {
    let fill_slot = |max_w| SlotParams {
        size_main: SizeModeMain::Fill(1.0),
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::new(0.0, max_w, 0.0, f32::INFINITY),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    };
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: fill_slot(20.0),
                child: LayoutNode::widget(2, Vector2::new(10.0, 10.0)),
            },
            SlotChild {
                slot: fill_slot(f32::INFINITY),
                child: LayoutNode::widget(3, Vector2::new(10.0, 10.0)),
            },
            SlotChild {
                slot: fill_slot(f32::INFINITY),
                child: LayoutNode::widget(4, Vector2::new(10.0, 10.0)),
            },
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 40.0)),
    );

    assert!((output.rects[&2].width() - 20.0).abs() < 0.5);
    assert!((output.rects[&3].width() - 50.0).abs() < 0.5);
    assert!((output.rects[&4].width() - 50.0).abs() < 0.5);
}

#[test]
fn row_rtl_uses_logical_start_and_swaps_physical_margins() {
    let slot = |left, right| SlotParams {
        size_main: SizeModeMain::Fixed(10.0),
        size_cross: SizeModeCross::Fixed(10.0),
        constraints: Constraints::unconstrained(),
        margin: crate::layout::Insets {
            left,
            right,
            ..Default::default()
        },
        align_cross_override: None,
        allow_fixed_compress: false,
    };
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Row,
            spacing: 4.0,
            ..Default::default()
        },
        vec![
            SlotChild {
                slot: slot(2.0, 5.0),
                child: LayoutNode::widget(2, Vector2::new(10.0, 10.0)),
            },
            SlotChild {
                slot: slot(7.0, 3.0),
                child: LayoutNode::widget(3, Vector2::new(10.0, 10.0)),
            },
        ],
    );
    let viewport = Rect::from_size(100.0, 20.0);
    let ltr = crate::layout::layout_tree_with_direction(&root, viewport, WritingDirection::Ltr);
    let rtl = crate::layout::layout_tree_with_direction(&root, viewport, WritingDirection::Rtl);
    assert_eq!(ltr.rects[&2].min.x, 2.0);
    assert_eq!(rtl.rects[&2].min.x, 85.0);
    assert_eq!(ltr.rects[&3].min.x, 28.0);
    assert_eq!(rtl.rects[&3].min.x, 66.0);
}

#[test]
fn column_and_stack_rtl_mirror_horizontal_cross_start() {
    let slot = SlotParams {
        size_main: SizeModeMain::Fixed(10.0),
        size_cross: SizeModeCross::Fixed(10.0),
        constraints: Constraints::unconstrained(),
        margin: crate::layout::Insets {
            left: 2.0,
            right: 5.0,
            ..Default::default()
        },
        align_cross_override: None,
        allow_fixed_compress: false,
    };
    let viewport = Rect::from_size(100.0, 40.0);
    let column = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            align_cross: crate::layout::CrossAlign::Start,
            ..Default::default()
        },
        vec![SlotChild {
            slot,
            child: LayoutNode::widget(2, Vector2::new(10.0, 10.0)),
        }],
    );
    let stack = LayoutNode::container(
        3,
        ContainerPolicy {
            kind: ContainerKind::Stack,
            align_cross: crate::layout::CrossAlign::Start,
            ..Default::default()
        },
        vec![SlotChild {
            slot,
            child: LayoutNode::widget(4, Vector2::new(10.0, 10.0)),
        }],
    );
    let column_ltr =
        crate::layout::layout_tree_with_direction(&column, viewport, WritingDirection::Ltr);
    let column_rtl =
        crate::layout::layout_tree_with_direction(&column, viewport, WritingDirection::Rtl);
    let stack_ltr =
        crate::layout::layout_tree_with_direction(&stack, viewport, WritingDirection::Ltr);
    let stack_rtl =
        crate::layout::layout_tree_with_direction(&stack, viewport, WritingDirection::Rtl);
    assert_eq!(column_ltr.rects[&2].min.x, 2.0);
    assert_eq!(column_rtl.rects[&2].min.x, 85.0);
    assert_eq!(stack_ltr.rects[&4].min.x, 2.0);
    assert_eq!(stack_rtl.rects[&4].min.x, 85.0);
}

#[test]
fn align_box_rtl_mirrors_horizontal_start() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::AlignBox,
            align_cross: crate::layout::CrossAlign::Start,
            ..Default::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(10.0),
                size_cross: SizeModeCross::Fixed(10.0),
                constraints: Constraints::unconstrained(),
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(2, Vector2::new(10.0, 10.0)),
        }],
    );
    let viewport = Rect::from_size(100.0, 40.0);
    let ltr = crate::layout::layout_tree_with_direction(&root, viewport, WritingDirection::Ltr);
    let rtl = crate::layout::layout_tree_with_direction(&root, viewport, WritingDirection::Rtl);
    assert_eq!(ltr.rects[&2].min.x, 0.0);
    assert_eq!(rtl.rects[&2].min.x, 90.0);
}
