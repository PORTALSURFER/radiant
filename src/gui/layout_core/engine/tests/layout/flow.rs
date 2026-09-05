use super::super::super::{layout_tree, tests::intrinsic_slot};
use crate::gui::layout_core::WritingDirection;
use crate::gui::{
    layout_core::{
        model::{ContainerKind, ContainerPolicy, GridPolicy, WrapPolicy},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

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

#[test]
fn wrap_rtl_keeps_source_order_from_the_logical_start() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: WrapPolicy {
                item_gap: 0.0,
                line_gap: 4.0,
            },
            ..Default::default()
        },
        (0..2)
            .map(|index| SlotChild {
                slot: intrinsic_slot(),
                child: LayoutNode::widget(index + 2, Vector2::new(30.0, 12.0)),
            })
            .collect(),
    );
    let output = crate::layout::layout_tree_with_direction(
        &root,
        Rect::from_size(100.0, 40.0),
        WritingDirection::Rtl,
    );
    assert_eq!(output.rects[&2].max.x, 100.0);
    assert!(output.rects[&3].max.x <= output.rects[&2].min.x);
}

#[test]
fn grid_rtl_places_logical_column_zero_on_the_right() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Grid,
            grid: GridPolicy {
                columns: 2,
                ..Default::default()
            },
            ..Default::default()
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
        ],
    );
    let output = crate::layout::layout_tree_with_direction(
        &root,
        Rect::from_size(100.0, 40.0),
        WritingDirection::Rtl,
    );
    assert!(output.rects[&2].min.x > output.rects[&3].min.x);
}
