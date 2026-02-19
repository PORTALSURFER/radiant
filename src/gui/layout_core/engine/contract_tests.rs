//! Contract tests for debug overlays, dirty propagation, and rounding behavior.

use super::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutEngine, LayoutState, layout_tree, round_rect,
};
use crate::gui::layout_core::model::{ContainerKind, ContainerPolicy, SlotParams};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::BTreeSet;

#[test]
fn measured_bounds_debug_primitives_are_emitted_when_enabled() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams::fill(),
            child: LayoutNode::widget(2, Vector2::new(40.0, 18.0)),
        }],
    );
    let mut engine = LayoutEngine::default();
    let output = engine.layout_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 48.0)),
        &LayoutState::default(),
        LayoutDebugOptions {
            enabled: true,
            show_bounds: false,
            show_measured: true,
            show_padding: false,
            show_margins: false,
            show_overflow: false,
        },
    );
    assert!(
        output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::MeasuredBounds)
    );
}

#[test]
fn measured_bounds_debug_primitives_are_omitted_when_disabled() {
    let root = LayoutNode::widget(1, Vector2::new(24.0, 12.0));
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 30.0)),
    );
    assert!(
        !output
            .debug_primitives
            .iter()
            .any(|item| item.kind == DebugPrimitiveKind::MeasuredBounds)
    );
}

#[test]
fn layout_dirty_subtree_filters_debug_output_to_marked_branch() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::container(
                    2,
                    ContainerPolicy {
                        kind: ContainerKind::Column,
                        ..ContainerPolicy::default()
                    },
                    vec![SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(3, Vector2::new(10.0, 10.0)),
                    }],
                ),
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(4, Vector2::new(10.0, 10.0)),
            },
        ],
    );
    let mut engine = LayoutEngine::default();
    engine.mark_layout_dirty_subtree(&root, 2);
    let output = engine.layout_with_state(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
        &LayoutState::default(),
        LayoutDebugOptions {
            enabled: true,
            show_bounds: true,
            show_measured: false,
            show_padding: false,
            show_margins: false,
            show_overflow: false,
        },
    );
    let nodes = output
        .debug_primitives
        .iter()
        .filter(|item| item.kind == DebugPrimitiveKind::NodeBounds)
        .map(|item| item.node_id)
        .collect::<BTreeSet<_>>();
    assert!(nodes.contains(&1));
    assert!(nodes.contains(&2));
    assert!(nodes.contains(&3));
    assert!(!nodes.contains(&4));
}

#[test]
fn measure_dirty_subtree_forces_branch_remeasure() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::container(
                    2,
                    ContainerPolicy {
                        kind: ContainerKind::Column,
                        ..ContainerPolicy::default()
                    },
                    vec![SlotChild {
                        slot: SlotParams::fill(),
                        child: LayoutNode::widget(3, Vector2::new(10.0, 10.0)),
                    }],
                ),
            },
            SlotChild {
                slot: SlotParams::fill(),
                child: LayoutNode::widget(4, Vector2::new(10.0, 10.0)),
            },
        ],
    );
    let mut engine = LayoutEngine::default();
    let _first = engine.layout(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    let second = engine.layout(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    assert_eq!(second.stats.measured_nodes, 0);

    engine.mark_measure_dirty_subtree(&root, 2);
    let third = engine.layout(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    assert!(third.stats.measured_nodes > 0);
    assert_ne!(third.stats.measured_nodes, second.stats.measured_nodes);
}

#[test]
fn round_rect_uses_stable_floor_origin_round_size_contract() {
    let input = Rect::from_min_size(Point::new(3.9, 4.2), Vector2::new(7.49, 2.51));
    let rounded = round_rect(input);
    assert_eq!(rounded.min.x, 3.0);
    assert_eq!(rounded.min.y, 4.0);
    assert_eq!(rounded.width(), 7.0);
    assert_eq!(rounded.height(), 3.0);
    assert_eq!(round_rect(input), rounded);
}
