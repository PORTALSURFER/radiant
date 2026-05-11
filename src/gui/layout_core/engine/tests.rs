//! Unit tests for strict slot layout engine behavior.

use super::{
    DebugPrimitiveKind, LayoutDebugOptions, LayoutDiagnosticCode, LayoutEngine, LayoutState,
    layout_tree, layout_tree_with_state,
};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{
    ContainerKind, ContainerPolicy, GridPolicy, OverflowPolicy, SizeModeCross, SizeModeMain,
    SlotParams, SwitchBreakpoint, VirtualizationAxis, VirtualizationPolicy, WrapPolicy,
};
use crate::gui::layout_core::tree::{LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

fn intrinsic_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Intrinsic,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

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
fn negative_widget_intrinsic_emits_diagnostic() {
    let root = LayoutNode::widget(1, Vector2::new(-32.0, 24.0));
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::NegativeSizeClamped)
    );
}

#[test]
fn contradictory_constraints_emit_diagnostic() {
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::Column,
            ..ContainerPolicy::default()
        },
        vec![SlotChild {
            slot: SlotParams {
                size_main: SizeModeMain::Fixed(10.0),
                size_cross: SizeModeCross::Fill,
                constraints: Constraints {
                    min_w: 40.0,
                    max_w: 20.0,
                    min_h: 5.0,
                    max_h: 2.0,
                },
                margin: Default::default(),
                align_cross_override: None,
                allow_fixed_compress: false,
            },
            child: LayoutNode::widget(2, Vector2::new(8.0, 8.0)),
        }],
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 80.0)),
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|item| item.code == LayoutDiagnosticCode::ConstraintContradiction)
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

#[test]
fn layout_engine_reuses_scratch_maps_between_passes() {
    let children = (0..64)
        .map(|index| {
            SlotChild::new(
                SlotParams {
                    size_main: SizeModeMain::Fixed(12.0),
                    size_cross: SizeModeCross::Fill,
                    constraints: Constraints::unconstrained(),
                    margin: Default::default(),
                    align_cross_override: None,
                    allow_fixed_compress: false,
                },
                LayoutNode::widget(index + 10, Vector2::new(40.0, 12.0)),
            )
        })
        .collect();
    let root = LayoutNode::container(
        1,
        ContainerPolicy {
            kind: ContainerKind::ScrollView,
            overflow: OverflowPolicy::Scroll,
            virtualization: Some(VirtualizationPolicy {
                enabled: true,
                axis: VirtualizationAxis::Vertical,
                overscan_px: 8.0,
            }),
            ..ContainerPolicy::default()
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Column,
                    ..ContainerPolicy::default()
                },
                children,
            ),
        )],
    );
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 64.0));
    let mut engine = LayoutEngine::default();

    let first = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );
    assert!(first.virtual_windows.contains_key(&1));
    let measured_capacity = engine.scratch.measured.capacity();
    let measured_by_node_capacity = engine.scratch.measured_by_node.capacity();
    let linear_window_capacity = engine.scratch.linear_windows.capacity();
    assert!(measured_capacity > 0);
    assert!(measured_by_node_capacity > 0);
    assert!(linear_window_capacity > 0);

    let second = engine.layout_with_state(
        &root,
        viewport,
        &LayoutState::default(),
        LayoutDebugOptions::default(),
    );

    assert!(second.virtual_windows.contains_key(&1));
    assert!(engine.scratch.measured.capacity() >= measured_capacity);
    assert!(engine.scratch.measured_by_node.capacity() >= measured_by_node_capacity);
    assert!(engine.scratch.linear_windows.capacity() >= linear_window_capacity);
}
