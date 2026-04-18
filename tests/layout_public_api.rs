//! Public API coverage for `radiant::layout`.

use radiant::layout::{
    Constraints, ContainerKind, ContainerPolicy, CrossAlign, Insets, LayoutEngine, LayoutNode,
    LayoutState, Point, Rect, SizeModeCross, SizeModeMain, SlotChild, SlotParams, Vector2,
    layout_tree,
};

#[test]
fn public_layout_module_supports_generic_tree_construction() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            padding: Insets::all(4.0),
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(20.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::new(0.0, 200.0, 20.0, 20.0),
                    margin: Insets::default(),
                    align_cross_override: Some(CrossAlign::Stretch),
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(2, Vector2::new(40.0, 20.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(3, Vector2::new(60.0, 30.0)),
            ),
        ],
    );

    let root_rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 90.0));
    let one_shot = layout_tree(&root, root_rect);

    let mut engine = LayoutEngine::default();
    let stateful = engine.layout_with_state(
        &root,
        root_rect,
        &LayoutState::default(),
        Default::default(),
    );

    assert_eq!(one_shot.rects, stateful.rects);
    assert!(one_shot.rects.contains_key(&2));
    assert!(one_shot.rects.contains_key(&3));
}
