use crate::gui::layout_core::{
    Constraints, LayoutDebugOptions, LayoutDiagnosticCode, LayoutEngine, LayoutNode,
    LayoutOmissionReason, LayoutPolicy, LayoutPolicyPlacementError, SizeHint, SlotChild,
    SlotParams, Vector2,
};
use crate::gui::types::{Point, Rect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct CountingPolicy {
    measure_calls: Rc<Cell<u32>>,
}

impl LayoutPolicy for CountingPolicy {
    fn measure(
        &self,
        children: &mut crate::gui::layout_core::MeasureChildren<'_>,
        constraints: Constraints,
    ) -> SizeHint {
        self.measure_calls.set(self.measure_calls.get() + 1);
        let _ = children.measure(0, constraints);
        SizeHint::preferred(Vector2::new(20.0, 12.0))
    }

    fn place(&self, children: &mut crate::gui::layout_core::PlaceChildren<'_>, bounds: Rect) {
        children
            .place(0, bounds)
            .expect("the only child should place");
    }
}

#[test]
fn custom_policy_measure_and_place_are_recomputed_without_cache_reuse() {
    let measure_calls = Rc::new(Cell::new(0));
    let root = LayoutNode::custom_container(
        1,
        CountingPolicy {
            measure_calls: Rc::clone(&measure_calls),
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::widget(2, Vector2::new(8.0, 8.0)),
        )],
    );
    let mut engine = LayoutEngine::default();
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0));

    let first = engine.layout(&root, bounds);
    let second = engine.layout(&root, bounds);

    assert_eq!(measure_calls.get(), 2);
    assert_eq!(first.rects.get(&2), Some(&bounds));
    assert_eq!(second.rects, first.rects);
}

#[test]
fn custom_policy_does_not_replace_first_accepted_placement() {
    struct PlacementPolicy;

    impl LayoutPolicy for PlacementPolicy {
        fn measure(
            &self,
            _children: &mut crate::gui::layout_core::MeasureChildren<'_>,
            _constraints: Constraints,
        ) -> SizeHint {
            SizeHint::preferred(Vector2::new(10.0, 10.0))
        }

        fn place(&self, children: &mut crate::gui::layout_core::PlaceChildren<'_>, _bounds: Rect) {
            let first = Rect::from_xy_size(3.0, 4.0, 12.0, 8.0);
            let replacement = Rect::from_xy_size(30.0, 40.0, 12.0, 8.0);
            assert_eq!(children.place(0, first), Ok(()));
            assert_eq!(
                children.place(0, replacement),
                Err(LayoutPolicyPlacementError::DuplicateDisposition { index: 0 })
            );
            assert_eq!(
                children.omit(4, LayoutOmissionReason::Conditional),
                Err(LayoutPolicyPlacementError::InvalidIndex {
                    index: 4,
                    child_count: 1,
                })
            );
        }
    }

    let root = LayoutNode::custom_container(
        10,
        PlacementPolicy,
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::widget(11, Vector2::new(8.0, 8.0)),
        )],
    );
    let output = crate::gui::layout_core::layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0)),
    );

    assert_eq!(
        output.rects.get(&11),
        Some(&Rect::from_xy_size(3.0, 4.0, 12.0, 8.0))
    );
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == crate::gui::layout_core::LayoutDiagnosticCode::CustomLayoutDuplicatePlacement
    }));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == crate::gui::layout_core::LayoutDiagnosticCode::CustomLayoutInvalidChildIndex
    }));
}

#[test]
fn custom_policy_child_measurement_stays_within_disjoint_slot_bounds() {
    struct RecordingPolicy {
        observed: Rc<RefCell<Vec<Constraints>>>,
    }

    impl LayoutPolicy for RecordingPolicy {
        fn measure(
            &self,
            _children: &mut crate::gui::layout_core::MeasureChildren<'_>,
            constraints: Constraints,
        ) -> SizeHint {
            self.observed.borrow_mut().push(constraints);
            SizeHint::preferred(Vector2::new(1.0, 1.0))
        }

        fn place(&self, _children: &mut crate::gui::layout_core::PlaceChildren<'_>, _bounds: Rect) {
        }
    }

    struct DisjointRequestPolicy;

    impl LayoutPolicy for DisjointRequestPolicy {
        fn measure(
            &self,
            children: &mut crate::gui::layout_core::MeasureChildren<'_>,
            _constraints: Constraints,
        ) -> SizeHint {
            children
                .measure(
                    0,
                    Constraints {
                        min_w: 80.0,
                        max_w: 100.0,
                        min_h: 80.0,
                        max_h: 100.0,
                    },
                )
                .expect("the only child should measure");
            SizeHint::preferred(Vector2::new(10.0, 10.0))
        }

        fn place(&self, children: &mut crate::gui::layout_core::PlaceChildren<'_>, bounds: Rect) {
            children
                .place(0, bounds)
                .expect("the only child should place");
        }
    }

    let observed = Rc::new(RefCell::new(Vec::new()));
    let slot = Constraints {
        min_w: 0.0,
        max_w: 40.0,
        min_h: 0.0,
        max_h: 40.0,
    };
    let root = LayoutNode::custom_container(
        20,
        DisjointRequestPolicy,
        vec![SlotChild::new(
            SlotParams {
                constraints: slot,
                ..SlotParams::fill()
            },
            LayoutNode::custom_container(
                21,
                RecordingPolicy {
                    observed: Rc::clone(&observed),
                },
                Vec::new(),
            ),
        )],
    );

    let _ = crate::gui::layout_core::layout_tree(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(120.0, 120.0)),
    );

    let observed = observed.borrow();
    assert_eq!(
        observed.as_slice(),
        &[Constraints {
            min_w: 0.0,
            max_w: 40.0,
            min_h: 0.0,
            max_h: 40.0,
        }]
    );
    assert!(observed.iter().all(|request| {
        request.min_w >= slot.min_w
            && request.max_w <= slot.max_w
            && request.min_h >= slot.min_h
            && request.max_h <= slot.max_h
    }));
}

#[test]
fn custom_policy_place_is_skipped_for_invalid_padding_geometry() {
    struct PlacementPolicy {
        place_calls: Rc<Cell<u32>>,
    }

    impl LayoutPolicy for PlacementPolicy {
        fn measure(
            &self,
            children: &mut crate::gui::layout_core::MeasureChildren<'_>,
            constraints: Constraints,
        ) -> SizeHint {
            let _ = children.measure(0, constraints);
            SizeHint::preferred(Vector2::new(20.0, 12.0))
        }

        fn place(&self, children: &mut crate::gui::layout_core::PlaceChildren<'_>, bounds: Rect) {
            self.place_calls.set(self.place_calls.get() + 1);
            children
                .place(0, bounds)
                .expect("the only child should place");
        }
    }

    let place_calls = Rc::new(Cell::new(0));
    let mut root = LayoutNode::custom_container(
        30,
        PlacementPolicy {
            place_calls: Rc::clone(&place_calls),
        },
        vec![SlotChild::new(
            SlotParams::fill(),
            LayoutNode::widget(31, Vector2::new(8.0, 8.0)),
        )],
    );
    if let LayoutNode::Container(container) = &mut root {
        container.policy.padding = crate::gui::layout_core::model::Insets::all(f32::NAN);
    }

    let output = crate::gui::layout_core::engine::layout_tree_with_state(
        &root,
        Rect::from_min_size(Point::default(), Vector2::new(80.0, 40.0)),
        &crate::gui::layout_core::engine::LayoutState::default(),
        LayoutDebugOptions {
            enabled: true,
            show_bounds: true,
            show_padding: true,
            show_margins: true,
            ..LayoutDebugOptions::default()
        },
    );

    assert_eq!(place_calls.get(), 0);
    assert!(output.rects.contains_key(&30));
    assert!(!output.rects.contains_key(&31));
    let diagnostics: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == LayoutDiagnosticCode::NegativeSizeClamped)
        .collect();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].message,
        "container content geometry was invalid and descendants were omitted"
    );
    assert!(output.debug_primitives.iter().all(|primitive| {
        primitive.rect.is_finite()
            && primitive.rect.width() >= 0.0
            && primitive.rect.height() >= 0.0
    }));
}
