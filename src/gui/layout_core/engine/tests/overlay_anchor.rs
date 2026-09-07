use super::intrinsic_slot;
use crate::gui::layout_core::{
    ContainerKind, ContainerPolicy, LayoutDebugOptions, LayoutNode, LayoutState, OverlayAnchor,
    Point, Rect, SlotChild, SlotParams, Vector2, layout_tree, layout_tree_with_state,
};

fn stack_policy() -> ContainerPolicy {
    ContainerPolicy {
        kind: ContainerKind::Stack,
        ..ContainerPolicy::default()
    }
}

fn viewport() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 100.0))
}

fn anchored_group(id: u64, target: u64) -> LayoutNode {
    LayoutNode::container(
        id,
        stack_policy(),
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(id + 1, Vector2::new(1.0, 1.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(id + 2, Vector2::new(1.0, 1.0)),
            ),
        ],
    )
    .with_overlay_anchor(OverlayAnchor::below(target, Vector2::new(50.0, 30.0)), true)
}

#[test]
fn current_pass_anchor_places_foreground_and_keeps_input_with_group() {
    let root = LayoutNode::container(
        1,
        stack_policy(),
        vec![
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(SlotParams::fill(), anchored_group(10, 2)),
        ],
    );
    let output = layout_tree(&root, viewport());

    assert_eq!(output.rects.get(&10), Some(&viewport()));
    assert!(
        output
            .rects
            .get(&11)
            .is_some_and(|rect| *rect == viewport())
    );
    assert!(
        output
            .rects
            .get(&12)
            .is_some_and(|rect| rect.width() == 50.0 && rect.height() == 30.0)
    );
}

#[test]
fn missing_or_duplicate_trigger_omits_the_complete_overlay_group() {
    let missing = LayoutNode::container(
        1,
        stack_policy(),
        vec![SlotChild::new(SlotParams::fill(), anchored_group(10, 999))],
    );
    let missing_output = layout_tree(&missing, viewport());
    for id in [10, 11, 12] {
        assert!(!missing_output.rects.contains_key(&id));
    }

    let duplicate = LayoutNode::container(
        1,
        stack_policy(),
        vec![
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(SlotParams::fill(), anchored_group(10, 2)),
        ],
    );
    let duplicate_output = layout_tree(&duplicate, viewport());
    for id in [10, 11, 12] {
        assert!(!duplicate_output.rects.contains_key(&id));
    }
}

#[test]
fn scroll_clipping_uses_the_current_target_visibility() {
    let scroll_content = LayoutNode::container(
        2,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(3, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(4, Vector2::new(20.0, 220.0)),
            ),
        ],
    );
    let root = LayoutNode::container(
        1,
        stack_policy(),
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::container(
                    5,
                    ContainerPolicy {
                        kind: ContainerKind::ScrollView,
                        overflow: crate::layout::OverflowPolicy::Scroll,
                        ..ContainerPolicy::default()
                    },
                    vec![SlotChild::new(intrinsic_slot(), scroll_content)],
                ),
            ),
            SlotChild::new(SlotParams::fill(), anchored_group(10, 3)),
        ],
    );

    let visible = layout_tree(&root, viewport());
    assert!(visible.rects.contains_key(&10));

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(5, Vector2::new(0.0, 5.0));
    let partial = layout_tree_with_state(&root, viewport(), &state, LayoutDebugOptions::default());
    assert!(partial.rects.contains_key(&10));
    assert_eq!(partial.rects[&12].min.y, partial.rects[&3].max.y);

    state.scroll_offsets.insert(5, Vector2::new(0.0, 20.0));
    let hidden = layout_tree_with_state(&root, viewport(), &state, LayoutDebugOptions::default());
    for id in [10, 11, 12] {
        assert!(!hidden.rects.contains_key(&id));
    }
}

#[test]
fn forward_and_self_referential_anchors_are_omitted() {
    let forward = LayoutNode::container(
        1,
        stack_policy(),
        vec![
            SlotChild::new(SlotParams::fill(), anchored_group(10, 2)),
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
        ],
    );
    let forward_output = layout_tree(&forward, viewport());
    assert!(!forward_output.rects.contains_key(&10));

    let self_referential = anchored_group(10, 10);
    let self_output = layout_tree(&self_referential, viewport());
    for id in [10, 11, 12] {
        assert!(!self_output.rects.contains_key(&id));
    }
}

#[test]
fn clip_depth_exhaustion_cannot_be_revived_by_a_later_duplicate_target() {
    let mut deeply_clipped = LayoutNode::widget(2, Vector2::new(20.0, 10.0));
    for id in 100..165 {
        deeply_clipped = LayoutNode::container(
            id,
            stack_policy(),
            vec![SlotChild::new(SlotParams::fill(), deeply_clipped)],
        );
    }
    let root = LayoutNode::container(
        1,
        stack_policy(),
        vec![
            SlotChild::new(SlotParams::fill(), deeply_clipped),
            SlotChild::new(
                intrinsic_slot(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(SlotParams::fill(), anchored_group(10, 2)),
        ],
    );

    let output = layout_tree(&root, viewport());
    for id in [10, 11, 12] {
        assert!(!output.rects.contains_key(&id));
    }
}
