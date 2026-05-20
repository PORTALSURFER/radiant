use super::super::layout_tree;
use super::intrinsic_slot;
use crate::gui::{
    layout_core::{
        constraints::Constraints,
        model::{
            ContainerKind, ContainerPolicy, GridPolicy, SizeModeCross, SizeModeMain, SlotParams,
            SwitchBreakpoint, WrapPolicy,
        },
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
fn switch_layout_selects_breakpoint_child() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::SwitchLayout,
            switch_breakpoints: vec![
                SwitchBreakpoint::new(0.0, 399.0),
                SwitchBreakpoint::new(400.0, 10_000.0),
            ],
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(2, Vector2::new(20.0, 20.0)),
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(3, Vector2::new(30.0, 30.0)),
            },
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(500.0, 100.0)),
    );
    assert!(!output.rects.contains_key(&2));
    assert!(output.rects.contains_key(&3));
}

#[test]
fn wrap_layout_moves_items_to_next_line() {
    let children = (0..3)
        .map(|index| SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(index + 2, Vector2::new(60.0, 12.0)),
        })
        .collect::<Vec<_>>();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: WrapPolicy {
                item_gap: 0.0,
                line_gap: 4.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 100.0)),
    );
    let first = output.rects.get(&2).expect("first item");
    let second = output.rects.get(&3).expect("second item");
    assert!(second.min.y > first.min.y);
}

#[test]
fn grid_layout_places_items_by_row_and_column() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Grid,
            grid: GridPolicy {
                columns: 2,
                column_gap: 10.0,
                row_gap: 5.0,
            },
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: intrinsic_slot(),
                child: LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            },
            SlotChild {
                slot: intrinsic_slot(),
                child: LayoutNode::widget(3, Vector2::new(20.0, 10.0)),
            },
            SlotChild {
                slot: intrinsic_slot(),
                child: LayoutNode::widget(4, Vector2::new(20.0, 10.0)),
            },
        ],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(110.0, 80.0)),
    );
    let a = output.rects.get(&2).expect("grid item 0");
    let b = output.rects.get(&3).expect("grid item 1");
    let c = output.rects.get(&4).expect("grid item 2");
    assert_eq!(a.min.y, b.min.y);
    assert!(b.min.x > a.min.x);
    assert!(c.min.y > a.min.y);
}
