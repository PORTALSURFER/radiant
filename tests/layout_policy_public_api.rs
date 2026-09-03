//! Public API coverage for the first declarative custom layout-policy slice.

use radiant::application::{IntoView, layout as layout_builder, text};
use radiant::layout::{
    Constraints, LayoutDiagnosticCode, LayoutNode, LayoutOmissionReason, LayoutPolicy,
    LayoutPolicyPlacementError, MeasureChildren, PlaceChildren, Point, Rect, SizeHint, SlotChild,
    SlotParams, Vector2, layout_tree,
};
use radiant::runtime::{SurfaceChild, SurfaceNode, UiSurface};
use radiant::widgets::{TextWidget, WidgetSizing};

struct PairPolicy;

impl LayoutPolicy for PairPolicy {
    fn measure(&self, children: &mut MeasureChildren<'_>, constraints: Constraints) -> SizeHint {
        let first = children
            .measure(0, constraints)
            .expect("first child exists");
        let second = children
            .measure(1, constraints)
            .expect("second child exists");
        SizeHint::new(
            Vector2::new(first.x + second.x, first.y.max(second.y)),
            Vector2::new(first.x + second.x + 4.0, first.y.max(second.y)),
        )
    }

    fn place(&self, children: &mut PlaceChildren<'_>, bounds: Rect) {
        let width = bounds.width() * 0.5;
        children
            .place(
                0,
                Rect::from_xy_size(bounds.min.x - 2.0, bounds.min.y, width, bounds.height()),
            )
            .expect("first child should place");
        children
            .place(
                1,
                Rect::from_xy_size(bounds.min.x + width, bounds.min.y, width, bounds.height()),
            )
            .expect("second child should place");
    }
}

fn assert_object_safe(_: &dyn LayoutPolicy) {}

fn assert_exhaustive_layout_diagnostic_code(code: LayoutDiagnosticCode) {
    match code {
        LayoutDiagnosticCode::NegativeSizeClamped
        | LayoutDiagnosticCode::ConstraintContradiction
        | LayoutDiagnosticCode::OverflowPolicyDefaulted
        | LayoutDiagnosticCode::OverflowOccurred
        | LayoutDiagnosticCode::InvalidScrollOffsetClamped
        | LayoutDiagnosticCode::VirtualizationPolicyIgnored
        | LayoutDiagnosticCode::VirtualizationWindowClamped
        | LayoutDiagnosticCode::VirtualizationAlignmentFallback
        | LayoutDiagnosticCode::VirtualizationSpanResolutionFallback
        | LayoutDiagnosticCode::SplitPaneChildCountMismatch
        | LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied
        | LayoutDiagnosticCode::CustomLayoutHintNonFinite
        | LayoutDiagnosticCode::CustomLayoutHintNegative
        | LayoutDiagnosticCode::CustomLayoutHintContradictory
        | LayoutDiagnosticCode::CustomLayoutInvalidChildIndex
        | LayoutDiagnosticCode::CustomLayoutInvalidPlacement
        | LayoutDiagnosticCode::CustomLayoutDuplicatePlacement
        | LayoutDiagnosticCode::CustomLayoutChildUnresolved => {}
    }
}

#[test]
fn public_layout_diagnostic_code_remains_exhaustive() {
    assert_exhaustive_layout_diagnostic_code(LayoutDiagnosticCode::NegativeSizeClamped);
}

#[test]
fn public_policy_is_object_safe_and_drives_direct_layout() {
    assert_object_safe(&PairPolicy);
    let root = LayoutNode::custom_container(
        1,
        PairPolicy,
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(2, Vector2::new(20.0, 10.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(3, Vector2::new(30.0, 12.0)),
            ),
        ],
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(100.0, 40.0)),
    );

    assert_eq!(output.rects[&2].min.x, -2.0);
    assert_eq!(output.rects[&2].width(), 50.0);
    assert_eq!(output.rects[&3].min.x, 50.0);
    assert_eq!(output.rects[&3].width(), 50.0);
}

struct MalformedPolicy;

impl LayoutPolicy for MalformedPolicy {
    fn measure(&self, children: &mut MeasureChildren<'_>, _constraints: Constraints) -> SizeHint {
        let _ = children.measure(
            0,
            Constraints {
                min_w: f32::NAN,
                max_w: -1.0,
                min_h: -2.0,
                max_h: 1.0,
            },
        );
        assert!(children.measure(9, Constraints::unconstrained()).is_err());
        SizeHint::new(Vector2::new(-1.0, f32::NAN), Vector2::new(8.0, 6.0))
            .with_maximum(Vector2::new(2.0, 1.0))
            .with_baseline(f32::INFINITY)
    }

    fn place(&self, children: &mut PlaceChildren<'_>, _bounds: Rect) {
        assert_eq!(
            children.place(
                0,
                Rect::from_min_max(Point::new(10.0, 10.0), Point::new(2.0, 2.0)),
            ),
            Err(LayoutPolicyPlacementError::InvalidRect {
                index: 0,
                rect: Rect::from_min_max(Point::new(10.0, 10.0), Point::new(2.0, 2.0))
            })
        );
        children
            .place(0, Rect::from_xy_size(4.0, 5.0, 20.0, 10.0))
            .expect("valid placement should be accepted");
        assert_eq!(
            children.place(0, Rect::from_size(1.0, 1.0)),
            Err(LayoutPolicyPlacementError::DuplicateDisposition { index: 0 })
        );
        children
            .omit(1, LayoutOmissionReason::Conditional)
            .expect("the second child should be explicitly omitted");
        assert_eq!(
            children.omit(3, LayoutOmissionReason::Unavailable),
            Err(LayoutPolicyPlacementError::InvalidIndex {
                index: 3,
                child_count: 3,
            })
        );
    }
}

#[test]
fn malformed_policy_inputs_are_diagnosed_and_unresolved_children_are_absent() {
    let root = LayoutNode::custom_container(
        10,
        MalformedPolicy,
        vec![
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(11, Vector2::new(8.0, 8.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(12, Vector2::new(8.0, 8.0)),
            ),
            SlotChild::new(
                SlotParams::fill(),
                LayoutNode::widget(13, Vector2::new(8.0, 8.0)),
            ),
        ],
    );
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0)),
    );

    for code in [
        LayoutDiagnosticCode::NegativeSizeClamped,
        LayoutDiagnosticCode::ConstraintContradiction,
        LayoutDiagnosticCode::CustomLayoutHintNonFinite,
        LayoutDiagnosticCode::CustomLayoutHintNegative,
        LayoutDiagnosticCode::CustomLayoutHintContradictory,
        LayoutDiagnosticCode::CustomLayoutInvalidChildIndex,
        LayoutDiagnosticCode::CustomLayoutInvalidPlacement,
        LayoutDiagnosticCode::CustomLayoutDuplicatePlacement,
    ] {
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code),
            "missing diagnostic {code:?}"
        );
    }
    assert_eq!(
        output.rects.get(&11),
        Some(&Rect::from_xy_size(4.0, 5.0, 20.0, 10.0))
    );
    assert!(!output.rects.contains_key(&12));
    assert!(!output.rects.contains_key(&13));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == LayoutDiagnosticCode::CustomLayoutChildUnresolved
    }));
}

#[test]
fn application_layout_builder_lowers_without_application_supplied_ids() {
    let surface =
        layout_builder(PairPolicy, [text::<()>("left"), text::<()>("right")]).into_surface();
    let root = surface.layout_node();
    let LayoutNode::Container(container) = &root else {
        panic!("custom application layout should lower to a container");
    };
    assert!(container.children.iter().all(|child| child.child.id() != 0));
    let output = layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(100.0, 30.0)),
    );
    assert!(output.rects.contains_key(&container.children[0].child.id()));
}

#[test]
fn runtime_surface_layout_builder_propagates_custom_policy() {
    let surface = UiSurface::new(SurfaceNode::layout(
        20,
        PairPolicy,
        vec![
            SurfaceChild::fill(SurfaceNode::<()>::static_widget(TextWidget::new(
                21,
                "left",
                WidgetSizing::fixed(Vector2::new(20.0, 10.0)),
            ))),
            SurfaceChild::fill(SurfaceNode::<()>::static_widget(TextWidget::new(
                22,
                "right",
                WidgetSizing::fixed(Vector2::new(20.0, 10.0)),
            ))),
        ],
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::default(), Vector2::new(100.0, 40.0)),
    );

    assert_eq!(output.rects[&21].min.x, -2.0);
    assert_eq!(output.rects[&22].min.x, 50.0);
}

#[test]
fn size_hint_accessors_return_normalized_values() {
    let hint = SizeHint::new(Vector2::new(-1.0, f32::NAN), Vector2::new(8.0, 6.0))
        .with_maximum(Vector2::new(2.0, 1.0))
        .with_baseline(-4.0);
    let minimum = hint.intrinsic_minimum();
    let preferred = hint.preferred_extent();
    let maximum = hint.maximum().expect("maximum was supplied");
    let baseline = hint.baseline().expect("baseline was supplied");
    assert!(minimum.x >= 0.0 && minimum.y >= 0.0);
    assert!(preferred.x >= minimum.x && preferred.y >= minimum.y);
    assert!(maximum.x >= preferred.x && maximum.y >= preferred.y);
    assert!(baseline >= 0.0 && baseline.is_finite());
}
