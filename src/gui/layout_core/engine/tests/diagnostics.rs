use super::super::{LayoutDiagnosticCode, layout_tree};
use crate::gui::{
    layout_core::{
        constraints::Constraints,
        model::{ContainerKind, ContainerPolicy, SizeModeCross, SizeModeMain, SlotParams},
        tree::{LayoutNode, SlotChild},
    },
    types::{Point, Rect, Vector2},
};

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
fn static_layout_diagnostics_borrow_messages() {
    let root = LayoutNode::widget(1, Vector2::new(-32.0, 24.0));
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|item| item.code == LayoutDiagnosticCode::NegativeSizeClamped)
        .expect("negative size should emit diagnostic");

    assert!(matches!(diagnostic.message, std::borrow::Cow::Borrowed(_)));
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
