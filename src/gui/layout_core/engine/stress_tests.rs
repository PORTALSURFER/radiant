//! Stress tests for deep and wide layout trees.

use super::layout_tree;
use crate::gui::layout_core::model::{ContainerKind, ContainerPolicy, SlotParams};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

#[test]
fn deep_nesting_layout_remains_stable() {
    let mut node = LayoutNode::widget(9_999, Vector2::new(8.0, 8.0));
    for id in (1..=300).rev() {
        node = LayoutNode::container(
            id,
            ContainerPolicy {
                kind: ContainerKind::PaddingBox,
                padding: crate::gui::layout_core::model::Insets::all(1.0),
                ..ContainerPolicy::default()
            },
            vec![SlotChild {
                slot: SlotParams::fill(),
                child: node,
            }],
        );
    }

    let output = layout_tree(
        &node,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
    );
    assert!(output.rects.len() >= 301);
    let deepest = output.rects.get(&9_999).expect("deepest widget");
    assert!(deepest.width() >= 0.0);
    assert!(deepest.height() >= 0.0);
}

#[test]
fn large_wrap_list_layout_produces_valid_rects() {
    let mut children = Vec::with_capacity(1_000);
    for index in 0..1_000_u64 {
        children.push(SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(index + 2, Vector2::new(12.0, 8.0)),
        });
    }

    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Wrap,
            wrap: crate::gui::layout_core::model::WrapPolicy {
                item_gap: 1.0,
                line_gap: 1.0,
            },
            ..ContainerPolicy::default()
        },
        children,
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1024.0, 768.0)),
    );

    assert_eq!(output.rects.len(), 1_001);
    for rect in output.rects.values() {
        assert!(rect.width().is_finite());
        assert!(rect.height().is_finite());
        assert!(rect.width() >= 0.0);
        assert!(rect.height() >= 0.0);
    }
}
