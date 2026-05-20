use super::super::{DebugPrimitiveKind, LayoutDebugOptions, LayoutEngine, LayoutState};
use crate::gui::{
    layout_core::{
        model::{ContainerKind, ContainerPolicy, SlotParams},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn debug_primitives_are_emitted_when_enabled() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            padding: crate::gui::layout_core::model::Insets::all(4.0),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(2, Vector2::new(30.0, 20.0)),
        }],
    );

    let mut engine = LayoutEngine::default();
    let output = engine.layout_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 50.0)),
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::NodeBounds)
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::ContentBounds)
    );
}
