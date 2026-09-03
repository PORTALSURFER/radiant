use super::super::{
    LayoutDebugOptions, LayoutDiagnosticCode, LayoutState, layout_tree, layout_tree_with_state,
};
use super::intrinsic_slot;
use crate::gui::{
    layout_core::{
        model::Insets,
        model::{ContainerKind, ContainerPolicy, OverflowPolicy, SlotParams},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

#[test]
fn malformed_padding_retains_container_and_omits_descendants() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            padding: Insets::all(f32::NAN),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(2, Vector2::new(8.0, 8.0)),
        }],
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0)),
    );
    assert!(output.rects.contains_key(&1));
    assert!(!output.rects.contains_key(&2));
    assert_eq!(
        output
            .diagnostics
            .iter()
            .filter(|item| item.code == LayoutDiagnosticCode::NegativeSizeClamped)
            .count(),
        1
    );
}

#[test]
fn malformed_margin_retains_scroll_container_and_omits_descendants() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: crate::gui::layout_core::model::SlotParams {
                margin: Insets::all(f32::NAN),
                ..SlotParams::fill()
            },
            child: LayoutNode::widget(2, Vector2::new(8.0, 8.0)),
        }],
    );
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0)),
        &LayoutState::default(),
        LayoutDebugOptions::all_enabled(),
    );
    assert!(output.rects.contains_key(&1));
    assert!(!output.rects.contains_key(&2));
    assert!(output.viewport_bounds.is_empty());
    assert!(output.virtual_windows.is_empty());
    assert!(output.overflowed.is_empty());
    assert!(output.overflow_flags.is_empty());
    let diagnostics: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|item| item.code == LayoutDiagnosticCode::NegativeSizeClamped)
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert!(output.diagnostics.iter().all(|item| {
        item.node_id == 1 && item.code == LayoutDiagnosticCode::NegativeSizeClamped
    }));
    assert!(output.debug_primitives.iter().all(|primitive| {
        primitive.node_id == 1
            && primitive.rect.is_finite()
            && primitive.rect.width() >= 0.0
            && primitive.rect.height() >= 0.0
    }));
}

#[test]
fn scroll_view_records_overflow_flags() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(2, Vector2::new(200.0, 160.0)),
        }],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );
    let overflow = output.overflow_flags.get(&1).expect("overflow info");
    assert!(overflow.x);
    assert!(overflow.y);
    assert_eq!(overflow.policy, OverflowPolicy::Scroll);
}

#[test]
fn scroll_view_records_padded_viewport_bounds() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            padding: Insets::all(4.0),
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(2, Vector2::new(200.0, 160.0)),
        }],
    );

    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
    );

    assert_eq!(
        output.viewport_bounds.get(&1),
        Some(&Rect::from_min_size(
            Point::new(4.0, 4.0),
            Vector2::new(92.0, 72.0)
        ))
    );
}

#[test]
fn scroll_offset_is_clamped_and_reported() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(2, Vector2::new(300.0, 200.0)),
        }],
    );

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(1000.0, -20.0));
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
        &state,
        LayoutDebugOptions::default(),
    );
    let child = output.rects.get(&2).expect("scroll content rect");
    assert_eq!(child.min.x, -200.0);
    assert_eq!(child.min.y, 0.0);
    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped)
    );
}

#[test]
fn positive_scroll_offsets_translate_content_left_and_up_on_both_axes() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: intrinsic_slot(),
            child: LayoutNode::widget(2, Vector2::new(300.0, 200.0)),
        }],
    );

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(1, Vector2::new(40.0, 25.0));
    let output = layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
        &state,
        LayoutDebugOptions::default(),
    );

    assert_eq!(
        output.rects.get(&2).expect("scroll content rect").min,
        Point::new(-40.0, -25.0)
    );
}
