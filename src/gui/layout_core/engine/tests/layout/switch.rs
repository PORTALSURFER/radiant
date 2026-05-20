use super::super::super::layout_tree;
use crate::gui::{
    layout_core::{
        model::{ContainerKind, ContainerPolicy, SlotParams, SwitchBreakpoint},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

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
