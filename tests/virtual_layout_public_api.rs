//! Public and object-safety coverage for the qualified virtual-layout query API.

use std::cell::Cell;

use radiant::{
    application::{VirtualLayoutParts, virtual_layout_from_parts},
    runtime::{
        VirtualLayoutRevisions, VirtualLayoutSemanticDeferredReason, VirtualLayoutSemanticEntry,
        VirtualLayoutSemanticProvider, VirtualLayoutSemanticProviderOutcome,
        VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeRequest,
        VirtualLayoutSemanticRequest, VirtualLayoutSemanticUnavailableReason,
    },
};

use radiant::layout::{
    NodeId, Point, Rect, Vector2, VirtualLayoutBoundsConfidence, VirtualLayoutBudget,
    VirtualLayoutCoordinateSpace, VirtualLayoutDeferredReason, VirtualLayoutDiagnosticCode,
    VirtualLayoutExtentCandidate, VirtualLayoutExtentKind, VirtualLayoutFenceField,
    VirtualLayoutItemCandidate, VirtualLayoutItemKey, VirtualLayoutOverscan, VirtualLayoutPolicy,
    VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity, VirtualLayoutQueryExecutor,
    VirtualLayoutQueryInput, VirtualLayoutQueryInputParts, VirtualLayoutQueryOutcome,
    VirtualLayoutQuerySink, VirtualLayoutUnavailableReason, VirtualLayoutVisibility,
};

fn input_parts() -> VirtualLayoutQueryInputParts {
    VirtualLayoutQueryInputParts {
        container_id: 41 as NodeId,
        policy_identity: VirtualLayoutPolicyIdentity::new("timeline"),
        mount_generation: 3,
        query_sequence: 4,
        viewport: Rect::from_xy_size(0.0, 10.0, 100.0, 80.0),
        coordinate_space: VirtualLayoutCoordinateSpace::logical(),
        overscan: VirtualLayoutOverscan::new(8.0, 12.0).expect("test overscan should be valid"),
        budget: VirtualLayoutBudget::new(4),
        viewport_revision: 11,
        data_revision: 12,
        policy_revision: 13,
        measurement_revision: 14,
        semantic_revision: 15,
    }
}

fn item(key: u32, logical_index: usize) -> VirtualLayoutItemCandidate {
    VirtualLayoutItemCandidate::new(
        VirtualLayoutItemKey::new(key),
        logical_index,
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 20.0)),
        VirtualLayoutVisibility::Visible,
        VirtualLayoutBoundsConfidence::Exact,
    )
}

struct OneItemPolicy;

impl VirtualLayoutPolicy for OneItemPolicy {
    fn query(
        &self,
        _input: &VirtualLayoutQueryInput,
        sink: &mut VirtualLayoutQuerySink,
    ) -> VirtualLayoutPolicyDecision {
        sink.visit(item(7, 0))
            .expect("the test budget admits one item");
        sink.set_extent(VirtualLayoutExtentCandidate::exact(Vector2::new(
            100.0, 20.0,
        )))
        .expect("the policy supplies one extent");
        VirtualLayoutPolicyDecision::Ready
    }
}

fn assert_object_safe(_: &dyn VirtualLayoutPolicy) {}

fn assert_public_provider_object_safe(
    _: &dyn VirtualLayoutSemanticProvider,
    _: &dyn VirtualLayoutSemanticRangeProvider,
) {
}

#[derive(Eq)]
struct UnstableIdentity {
    calls: Cell<u32>,
}

type FenceChange = fn(&mut VirtualLayoutQueryInputParts);

impl PartialEq for UnstableIdentity {
    fn eq(&self, _other: &Self) -> bool {
        let call = self.calls.get();
        self.calls.set(call.saturating_add(1));
        call.is_multiple_of(2)
    }
}

#[test]
fn qualified_policy_is_object_safe_and_returns_bounded_typed_result() {
    let policy = OneItemPolicy;
    assert_object_safe(&policy);

    let executor = VirtualLayoutQueryExecutor::from_parts(input_parts())
        .expect("the named query input should be valid");
    assert_eq!(executor.admitted_entry_budget(), 4);

    let VirtualLayoutQueryOutcome::Ready(result) = executor.execute(&policy) else {
        panic!("the bounded policy should produce a ready result");
    };
    assert_eq!(result.len(), 1);
    assert_eq!(result.entry(0).expect("one item").logical_index(), 0);
    assert_eq!(
        result.entry(0).expect("one item").key(),
        &VirtualLayoutItemKey::new(7_u32)
    );
    assert_eq!(result.extent().kind(), VirtualLayoutExtentKind::Exact);
    assert!(
        result
            .fence()
            .mismatched_fields(executor.fence())
            .is_empty()
    );
}

#[test]
fn exact_fences_reject_each_changed_field_without_ordering_comparison() {
    let first = VirtualLayoutQueryExecutor::from_parts(input_parts())
        .expect("the first query input should be valid");
    let VirtualLayoutQueryOutcome::Ready(result) = first.execute(&OneItemPolicy) else {
        panic!("the policy should produce a ready result");
    };

    let cases: &[(VirtualLayoutFenceField, FenceChange)] = &[
        (VirtualLayoutFenceField::ContainerIdentity, |parts| {
            parts.container_id += 1;
        }),
        (VirtualLayoutFenceField::PolicyIdentity, |parts| {
            parts.policy_identity = VirtualLayoutPolicyIdentity::new("other-policy");
        }),
        (VirtualLayoutFenceField::MountGeneration, |parts| {
            parts.mount_generation += 1;
        }),
        (VirtualLayoutFenceField::QuerySequence, |parts| {
            parts.query_sequence += 1;
        }),
        (VirtualLayoutFenceField::ViewportRevision, |parts| {
            parts.viewport_revision += 1;
        }),
        (VirtualLayoutFenceField::DataRevision, |parts| {
            parts.data_revision += 1;
        }),
        (VirtualLayoutFenceField::PolicyRevision, |parts| {
            parts.policy_revision += 1;
        }),
        (VirtualLayoutFenceField::MeasurementRevision, |parts| {
            parts.measurement_revision += 1;
        }),
        (VirtualLayoutFenceField::SemanticRevision, |parts| {
            parts.semantic_revision += 1;
        }),
        (VirtualLayoutFenceField::Viewport, |parts| {
            parts.viewport = Rect::from_xy_size(1.0, 10.0, 100.0, 80.0);
        }),
        (VirtualLayoutFenceField::CoordinateSpace, |parts| {
            parts.coordinate_space = VirtualLayoutCoordinateSpace::custom(
                VirtualLayoutPolicyIdentity::new("custom-space"),
            );
        }),
        (VirtualLayoutFenceField::Overscan, |parts| {
            parts.overscan =
                VirtualLayoutOverscan::new(9.0, 12.0).expect("test overscan should be valid");
        }),
        (VirtualLayoutFenceField::Budget, |parts| {
            parts.budget = VirtualLayoutBudget::new(3);
        }),
    ];

    for (field, change) in cases {
        let mut changed = input_parts();
        change(&mut changed);
        let second = VirtualLayoutQueryExecutor::from_parts(changed)
            .expect("the changed query input should be valid");
        let VirtualLayoutQueryOutcome::Invalid(diagnostics) = second.accept(result.clone()) else {
            panic!("changed fence field {field:?} must invalidate the result");
        };
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code() == VirtualLayoutDiagnosticCode::FenceMismatch
                && diagnostic.fence_fields().contains(*field)
        }));
    }
}

#[test]
fn unstable_policy_identity_cannot_pass_exact_fence() {
    let mut parts = input_parts();
    parts.policy_identity = VirtualLayoutPolicyIdentity::new(UnstableIdentity {
        calls: Cell::new(0),
    });
    let first = VirtualLayoutQueryExecutor::from_parts(parts.clone())
        .expect("the query input should be valid");
    let VirtualLayoutQueryOutcome::Ready(result) = first.execute(&OneItemPolicy) else {
        panic!("the policy should produce a ready result");
    };

    let second = VirtualLayoutQueryExecutor::from_parts(parts)
        .expect("the equivalent query input should be valid");
    assert!(matches!(
        second.accept(result),
        VirtualLayoutQueryOutcome::Invalid(diagnostics)
            if diagnostics.iter().any(|diagnostic| {
                diagnostic.code() == VirtualLayoutDiagnosticCode::FenceMismatch
                    && diagnostic
                        .fence_fields()
                        .contains(VirtualLayoutFenceField::PolicyIdentity)
            })
    ));
}

#[test]
fn unstable_custom_coordinate_identity_cannot_pass_exact_fence() {
    let mut parts = input_parts();
    parts.coordinate_space =
        VirtualLayoutCoordinateSpace::custom(VirtualLayoutPolicyIdentity::new(UnstableIdentity {
            calls: Cell::new(0),
        }));
    let first = VirtualLayoutQueryExecutor::from_parts(parts.clone())
        .expect("the query input should be valid");
    let VirtualLayoutQueryOutcome::Ready(result) = first.execute(&OneItemPolicy) else {
        panic!("the policy should produce a ready result");
    };

    let second = VirtualLayoutQueryExecutor::from_parts(parts)
        .expect("the equivalent query input should be valid");
    assert!(matches!(
        second.accept(result),
        VirtualLayoutQueryOutcome::Invalid(diagnostics)
            if diagnostics.iter().any(|diagnostic| {
                diagnostic.code() == VirtualLayoutDiagnosticCode::FenceMismatch
                    && diagnostic
                        .fence_fields()
                        .contains(VirtualLayoutFenceField::CoordinateSpace)
            })
    ));
}

#[test]
fn invalid_input_does_not_invoke_object_safe_policy() {
    struct CountingPolicy {
        calls: Cell<u32>,
    }

    impl VirtualLayoutPolicy for CountingPolicy {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            _sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.calls.set(self.calls.get() + 1);
            VirtualLayoutPolicyDecision::Ready
        }
    }

    let mut invalid = input_parts();
    invalid.viewport = Rect::from_min_max(Point::new(20.0, 0.0), Point::new(10.0, 1.0));
    let policy = CountingPolicy {
        calls: Cell::new(0),
    };
    assert!(matches!(
        VirtualLayoutQueryExecutor::execute_parts(invalid, &policy),
        VirtualLayoutQueryOutcome::Invalid(_)
    ));
    assert_eq!(policy.calls.get(), 0);
}

#[test]
fn policy_dispositions_are_distinct_and_the_capability_is_not_in_the_prelude() {
    struct Disposition(VirtualLayoutPolicyDecision);

    impl VirtualLayoutPolicy for Disposition {
        fn query(
            &self,
            _input: &VirtualLayoutQueryInput,
            _sink: &mut VirtualLayoutQuerySink,
        ) -> VirtualLayoutPolicyDecision {
            self.0
        }
    }

    let executor = VirtualLayoutQueryExecutor::from_parts(input_parts())
        .expect("the query input should be valid");
    assert!(matches!(
        executor.execute(&Disposition(VirtualLayoutPolicyDecision::Unavailable(
            VirtualLayoutUnavailableReason::DataUnavailable
        ))),
        VirtualLayoutQueryOutcome::Unavailable(VirtualLayoutUnavailableReason::DataUnavailable)
    ));
    assert!(matches!(
        executor.execute(&Disposition(VirtualLayoutPolicyDecision::Deferred(
            VirtualLayoutDeferredReason::DataPending
        ))),
        VirtualLayoutQueryOutcome::Deferred(VirtualLayoutDeferredReason::DataPending)
    ));

    let prelude_layout = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/layout.rs"
    ));
    assert!(!prelude_layout.contains("VirtualLayoutPolicy"));
}

#[test]
fn public_provider_attachment_is_qualified_and_has_no_prelude_or_runtime_leakage() {
    let item_provider: std::rc::Rc<dyn VirtualLayoutSemanticProvider> =
        std::rc::Rc::new(|_request: &VirtualLayoutSemanticRequest| {
            VirtualLayoutSemanticProviderOutcome::Found(VirtualLayoutSemanticEntry::new(
                VirtualLayoutItemKey::new(7_u32),
                0,
                Rect::from_xy_size(0.0, 0.0, 20.0, 20.0),
                radiant::gui::automation::AutomationNodeSemantics::new(
                    radiant::gui::automation::AutomationRole::Row,
                ),
                radiant::gui::automation::AutomationNodeId::new("public-provider"),
            ))
        });
    let range_provider: std::rc::Rc<dyn VirtualLayoutSemanticRangeProvider> =
        std::rc::Rc::new(|_request: &VirtualLayoutSemanticRangeRequest| {
            VirtualLayoutSemanticProviderOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported,
            )
        });
    assert_public_provider_object_safe(&*item_provider, &*range_provider);

    let _parts = VirtualLayoutParts::new(
        std::rc::Rc::new(OneItemPolicy),
        VirtualLayoutPolicyIdentity::new("qualified-provider-test"),
        VirtualLayoutOverscan::new(0.0, 0.0).expect("valid overscan"),
        VirtualLayoutBudget::new(1),
        VirtualLayoutRevisions::new(1, 2, 3, 4),
        std::rc::Rc::new(|| radiant::prelude::column::<()>([])),
        std::rc::Rc::new(|_item| radiant::prelude::text::<()>("item")),
        std::rc::Rc::new(|_item| VirtualLayoutPolicyIdentity::new("item-kind")),
    )
    .with_semantic_provider(item_provider)
    .with_semantic_range_provider(range_provider);
    let _ = virtual_layout_from_parts(_parts);

    assert_eq!(
        VirtualLayoutSemanticDeferredReason::Retry,
        VirtualLayoutSemanticDeferredReason::Retry
    );
    assert_eq!(VirtualLayoutRevisions::new(1, 2, 3, 4).semantic, 4);
    for source in [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prelude/application.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prelude/application/view.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prelude/runtime.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/prelude/runtime/commands.rs"
        )),
    ] {
        assert!(!source.contains("VirtualLayoutParts"));
        assert!(!source.contains("VirtualLayoutSemanticProvider"));
        assert!(!source.contains("VirtualLayoutSemanticRangeProvider"));
    }
}
