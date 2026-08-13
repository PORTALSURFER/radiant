use super::*;
use radiant::{
    application::{
        ViewNode, VirtualLayoutParts, virtual_layout::VirtualLayoutSemanticCardinality,
        virtual_layout_from_parts,
    },
    gui::automation::{AutomationNodeId, AutomationNodeSemantics, AutomationRole},
    layout::{
        VirtualLayoutBoundsConfidence, VirtualLayoutBudget, VirtualLayoutItemCandidate,
        VirtualLayoutItemKey, VirtualLayoutOverscan, VirtualLayoutPolicy,
        VirtualLayoutPolicyDecision, VirtualLayoutPolicyIdentity, VirtualLayoutQueryInput,
        VirtualLayoutQuerySink, VirtualLayoutVisibility,
    },
    runtime::virtual_layout::{
        VirtualLayoutSemanticCoordinateTransform, VirtualLayoutSemanticCoordinateTransformOutcome,
    },
    runtime::{
        RepaintScope, SemanticAutomationDemand, SemanticAutomationFallbackReason,
        SemanticAutomationRefreshStatus, SemanticAutomationSessionError, VirtualLayoutRevisions,
        VirtualLayoutSemanticDeferredReason, VirtualLayoutSemanticEntry,
        VirtualLayoutSemanticProvider, VirtualLayoutSemanticProviderOutcome,
        VirtualLayoutSemanticRangeProvider, VirtualLayoutSemanticRangeRequest,
        VirtualLayoutSemanticRequest, VirtualLayoutSemanticUnavailableReason,
    },
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct PublicProviderPolicy;

impl VirtualLayoutPolicy for PublicProviderPolicy {
    fn query(
        &self,
        _input: &VirtualLayoutQueryInput,
        sink: &mut VirtualLayoutQuerySink,
    ) -> VirtualLayoutPolicyDecision {
        sink.visit_item(VirtualLayoutItemCandidate::new(
            VirtualLayoutItemKey::new(7_u32),
            0,
            Rect::from_xy_size(0.0, 0.0, 120.0, 24.0),
            VirtualLayoutVisibility::Visible,
            VirtualLayoutBoundsConfidence::Exact,
        ))
        .expect("the public fixture item is within budget");
        sink.set_extent(radiant::layout::VirtualLayoutExtentCandidate::exact(
            Vector2::new(120.0, 24.0),
        ))
        .expect("the public fixture extent is unique");
        VirtualLayoutPolicyDecision::Ready
    }
}

fn semantic_entry(key: u32, index: usize, id: &str) -> VirtualLayoutSemanticEntry {
    VirtualLayoutSemanticEntry::new(
        VirtualLayoutItemKey::new(key),
        index,
        Rect::from_xy_size(0.0, index as f32 * 24.0, 120.0, 24.0),
        AutomationNodeSemantics::new(AutomationRole::Row)
            .with_label(format!("Provider row {key}"))
            .with_value_text(format!("value-{key}")),
        AutomationNodeId::new(id),
    )
}

fn public_parts_with(
    policy_scope: u32,
    revisions: VirtualLayoutRevisions,
    item_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
    range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
) -> VirtualLayoutParts<()> {
    let mut parts = VirtualLayoutParts::new(
        Rc::new(PublicProviderPolicy),
        VirtualLayoutPolicyIdentity::new(policy_scope),
        VirtualLayoutOverscan::new(0.0, 0.0).expect("valid fixture overscan"),
        VirtualLayoutBudget::new(4),
        revisions,
        Rc::new(|| ui::scroll(ui::spacer::<()>().size(120.0, 24.0))),
        Rc::new(|_item| ui::text::<()>("materialized item")),
        Rc::new(|_item| VirtualLayoutPolicyIdentity::new("public-item")),
    );
    parts.semantic_provider = item_provider;
    parts.semantic_range_provider = range_provider;
    parts
}

fn public_parts(
    range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
) -> VirtualLayoutParts<()> {
    public_parts_with(
        1,
        VirtualLayoutRevisions::new(1, 2, 3, 4),
        None,
        range_provider,
    )
}

fn public_custom_parts(
    shell: Rc<dyn Fn() -> ViewNode<()>>,
    provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
    transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform>,
) -> VirtualLayoutParts<()> {
    let mut parts = public_parts(Some(provider)).with_semantic_coordinate_transform(
        VirtualLayoutPolicyIdentity::new("degenerate-clip-space"),
        31,
        transform,
    );
    parts.shell = shell;
    parts
}

#[derive(Clone, Copy)]
enum DegenerateClipKind {
    OwnViewport,
    Ancestor,
}

fn custom_surface_for_clip_kind(
    kind: DegenerateClipKind,
    provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
    transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform>,
) -> radiant::runtime::UiSurface<()> {
    let shell = match kind {
        DegenerateClipKind::OwnViewport => {
            Rc::new(|| ui::scroll(ui::spacer::<()>().size(120.0, 24.0)).padding(120.0))
                as Rc<dyn Fn() -> ViewNode<()>>
        }
        DegenerateClipKind::Ancestor => {
            Rc::new(|| ui::scroll(ui::spacer::<()>().size(120.0, 24.0)))
                as Rc<dyn Fn() -> ViewNode<()>>
        }
    };
    let custom = virtual_layout_from_parts(public_custom_parts(shell, provider, transform));
    match kind {
        DegenerateClipKind::OwnViewport => custom.into_surface(),
        DegenerateClipKind::Ancestor => ui::row([
            ui::scroll(
                ui::row([
                    ui::spacer::<()>().width(120.0),
                    custom.width(1.0).height(1.0),
                ])
                .spacing(0.0),
            )
            .width(120.0)
            .height(100.0),
            ui::spacer::<()>().fill_width(),
        ])
        .spacing(0.0)
        .fill()
        .into_surface(),
    }
}

fn custom_surface_with_surface_intersection(
    degenerate: bool,
    provider: Rc<dyn VirtualLayoutSemanticRangeProvider>,
    transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform>,
) -> radiant::runtime::UiSurface<()> {
    let shell = Rc::new(|| ui::scroll(ui::spacer::<()>().size(120.0, 24.0)))
        as Rc<dyn Fn() -> ViewNode<()>>;
    let custom = virtual_layout_from_parts(public_custom_parts(shell, provider, transform))
        .width(1.0)
        .height(1.0);
    ui::row([
        ui::spacer::<()>().width(if degenerate { 240.0 } else { 0.0 }),
        custom,
    ])
    .spacing(0.0)
    .fill()
    .into_surface()
}

fn degenerate_clip_provider(calls: Rc<Cell<usize>>) -> Rc<dyn VirtualLayoutSemanticRangeProvider> {
    Rc::new(move |_request: &VirtualLayoutSemanticRangeRequest| {
        calls.set(calls.get() + 1);
        VirtualLayoutSemanticProviderOutcome::Found(vec![
            semantic_entry(7, 0, "degenerate-provider-7"),
            semantic_entry(8, 1, "degenerate-provider-8"),
        ])
    })
}

fn degenerate_clip_transform(
    calls: Rc<Cell<usize>>,
) -> Rc<dyn VirtualLayoutSemanticCoordinateTransform> {
    Rc::new(
        move |request: &radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransformRequest| {
            calls.set(calls.get() + 1);
            VirtualLayoutSemanticCoordinateTransformOutcome::Found(request.destination_clip())
        },
    )
}

#[derive(Clone, Copy)]
enum CustomProviderResponse {
    Found,
    DataUnavailable,
    Deferred,
}

#[derive(Clone, Copy)]
enum CustomTransformResponse {
    Unsupported,
    Singular,
    Ambiguous,
    Panic,
    Inverted,
    NonFinite,
    OutsideClip,
}

fn controlled_coordinate_transform(
    response: Rc<Cell<CustomTransformResponse>>,
    calls: Rc<Cell<usize>>,
) -> Rc<dyn VirtualLayoutSemanticCoordinateTransform> {
    Rc::new(
        move |request: &radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransformRequest| {
        calls.set(calls.get() + 1);
        match response.get() {
            CustomTransformResponse::Unsupported => {
                VirtualLayoutSemanticCoordinateTransformOutcome::Unsupported
            }
            CustomTransformResponse::Singular => {
                VirtualLayoutSemanticCoordinateTransformOutcome::Singular
            }
            CustomTransformResponse::Ambiguous => {
                VirtualLayoutSemanticCoordinateTransformOutcome::Ambiguous
            }
            CustomTransformResponse::Panic => panic!("custom transform panic fixture"),
            CustomTransformResponse::Inverted => {
                VirtualLayoutSemanticCoordinateTransformOutcome::Found(Rect::from_min_max(
                    Point::new(10.0, 10.0),
                    Point::new(1.0, 1.0),
                ))
            }
            CustomTransformResponse::NonFinite => {
                VirtualLayoutSemanticCoordinateTransformOutcome::Found(Rect::from_min_max(
                    Point::new(f32::NAN, 0.0),
                    Point::new(1.0, 1.0),
                ))
            }
            CustomTransformResponse::OutsideClip => {
                let clip = request.destination_clip();
                VirtualLayoutSemanticCoordinateTransformOutcome::Found(Rect::from_xy_size(
                    clip.max.x + 10.0,
                    clip.max.y + 10.0,
                    1.0,
                    1.0,
                ))
            }
        }
        },
    )
}

#[test]
fn custom_coordinate_attachment_maps_nonlinearly_only_during_explicit_refresh() {
    let provider_calls = Rc::new(Cell::new(0));
    let transform_calls = Rc::new(Cell::new(0));
    let provider_response = Rc::new(Cell::new(CustomProviderResponse::Found));
    let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> = Rc::new({
        let provider_calls = Rc::clone(&provider_calls);
        let provider_response = Rc::clone(&provider_response);
        move |_request: &VirtualLayoutSemanticRangeRequest| match provider_response.get() {
            CustomProviderResponse::Found => {
                provider_calls.set(provider_calls.get() + 1);
                VirtualLayoutSemanticProviderOutcome::Found(vec![
                    semantic_entry(7, 0, "custom-provider-7"),
                    semantic_entry(8, 1, "custom-provider-8"),
                ])
            }
            CustomProviderResponse::DataUnavailable => {
                provider_calls.set(provider_calls.get() + 1);
                VirtualLayoutSemanticProviderOutcome::Unavailable(
                    VirtualLayoutSemanticUnavailableReason::DataUnavailable,
                )
            }
            CustomProviderResponse::Deferred => {
                provider_calls.set(provider_calls.get() + 1);
                VirtualLayoutSemanticProviderOutcome::Deferred(
                    VirtualLayoutSemanticDeferredReason::SemanticPending,
                )
            }
        }
    });
    let transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform> = Rc::new({
        let transform_calls = Rc::clone(&transform_calls);
        move |request: &radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransformRequest| {
            transform_calls.set(transform_calls.get() + 1);
            assert_eq!(request.revisions(), VirtualLayoutRevisions::new(1, 2, 3, 4));
            assert_eq!(request.transform_revision(), 17);
            let source = request.source_rect();
            let anchor = request.ordinary_container_anchor();
            let min_x = anchor.min.x + source.min.x * source.min.x / 120.0;
            let max_x = anchor.min.x + source.max.x * source.max.x / 120.0;
            let min_y = anchor.min.y + source.min.y * 2.0;
            let max_y = request.destination_clip().max.y + 1.0;
            VirtualLayoutSemanticCoordinateTransformOutcome::Found(Rect::from_min_max(
                Point::new(min_x, min_y),
                Point::new(max_x, max_y),
            ))
        }
    });
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            let parts = public_parts(Some(Rc::clone(&provider)))
                .with_semantic_coordinate_transform(
                    VirtualLayoutPolicyIdentity::new("custom-space"),
                    17,
                    Rc::clone(&transform),
                );
            arc_surface(virtual_layout_from_parts(parts).into_surface())
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    assert_eq!(provider_calls.get(), 0);
    assert_eq!(transform_calls.get(), 0);
    let ordinary_before = runtime.automation_snapshot();
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    assert_eq!(runtime.automation_snapshot(), ordinary_before);
    assert_eq!(transform_calls.get(), 0);

    let refresh = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("custom transform refresh succeeds");
    assert_eq!(refresh.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(provider_calls.get(), 1);
    assert_eq!(transform_calls.get(), 2);
    let first = refresh
        .selected
        .targets
        .targets
        .iter()
        .find(|target| target.id.0 == "custom-provider-8")
        .expect("transformed provider entry is published");
    assert!(
        first.bounds.width > 20.0,
        "the nonlinear x envelope was used"
    );
    assert!(first.bounds.height >= 48.0);
    assert_eq!(first.bounds.height, 52.0);
    let selected = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("selected reads remain passive");
    assert_eq!(selected.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(provider_calls.get(), 1);
    assert_eq!(transform_calls.get(), 2);

    runtime.set_viewport(Vector2::new(180.0, 80.0));
    let refreshed_context = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("changed destination context refreshes the custom transform");
    assert_eq!(
        refreshed_context.status,
        SemanticAutomationRefreshStatus::Published
    );
    assert_eq!(provider_calls.get(), 2);
    assert_eq!(transform_calls.get(), 4);

    provider_response.set(CustomProviderResponse::DataUnavailable);
    let retained = runtime
        .retry_semantic_automation_session(session)
        .expect("data-unavailable custom retry retains the exact transformed selection");
    assert_eq!(
        retained.status,
        SemanticAutomationRefreshStatus::Retained {
            reason: SemanticAutomationFallbackReason::DataUnavailable,
        }
    );
    assert_eq!(provider_calls.get(), 3);
    assert_eq!(transform_calls.get(), 4);

    provider_response.set(CustomProviderResponse::Deferred);
    let deferred = runtime
        .retry_semantic_automation_session(session)
        .expect("deferred custom retry retains the exact transformed selection");
    assert_eq!(
        deferred.status,
        SemanticAutomationRefreshStatus::Retained {
            reason: SemanticAutomationFallbackReason::Deferred,
        }
    );
    assert_eq!(provider_calls.get(), 4);
    assert_eq!(transform_calls.get(), 4);
}

#[test]
fn custom_transform_failures_use_ordinary_baseline_without_partial_entries() {
    for response_kind in [
        CustomTransformResponse::Unsupported,
        CustomTransformResponse::Singular,
        CustomTransformResponse::Ambiguous,
        CustomTransformResponse::Panic,
        CustomTransformResponse::Inverted,
        CustomTransformResponse::NonFinite,
        CustomTransformResponse::OutsideClip,
    ] {
        let provider_calls = Rc::new(Cell::new(0));
        let transform_calls = Rc::new(Cell::new(0));
        let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> = Rc::new({
            let provider_calls = Rc::clone(&provider_calls);
            move |_request: &VirtualLayoutSemanticRangeRequest| {
                provider_calls.set(provider_calls.get() + 1);
                VirtualLayoutSemanticProviderOutcome::Found(vec![
                    semantic_entry(7, 0, "failed-transform-7"),
                    semantic_entry(8, 1, "failed-transform-8"),
                ])
            }
        });
        let transform = controlled_coordinate_transform(
            Rc::new(Cell::new(response_kind)),
            Rc::clone(&transform_calls),
        );
        let bridge = declarative_runtime_bridge(
            (),
            move |_state: &mut ()| {
                let parts = public_parts(Some(Rc::clone(&provider)))
                    .with_semantic_coordinate_transform(
                        VirtualLayoutPolicyIdentity::new("failure-space"),
                        19,
                        Rc::clone(&transform),
                    );
                arc_surface(virtual_layout_from_parts(parts).into_surface())
            },
            |_state: &mut (), _message: ()| {},
        );
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
        let (session, container) = start_range_session(&mut runtime);
        let publication = runtime
            .refresh_semantic_automation_session(
                session,
                &[SemanticAutomationDemand::range(container, 0, 2)],
            )
            .expect("transform failure is represented by a conservative publication");
        assert_eq!(
            publication.status,
            SemanticAutomationRefreshStatus::Baseline {
                reason: SemanticAutomationFallbackReason::Rejected,
            }
        );
        assert_eq!(provider_calls.get(), 1);
        assert_eq!(transform_calls.get(), 1);
        assert!(
            publication
                .selected
                .targets
                .targets
                .iter()
                .all(|target| !target.id.0.starts_with("failed-transform-"))
        );
    }
}

#[test]
fn edge_touching_custom_transform_uses_ordinary_baseline_without_custom_target() {
    let provider_calls = Rc::new(Cell::new(0));
    let transform_calls = Rc::new(Cell::new(0));
    let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> = Rc::new({
        let provider_calls = Rc::clone(&provider_calls);
        move |_request: &VirtualLayoutSemanticRangeRequest| {
            provider_calls.set(provider_calls.get() + 1);
            VirtualLayoutSemanticProviderOutcome::Found(vec![
                semantic_entry(7, 0, "edge-touching-provider-7"),
                semantic_entry(8, 1, "edge-touching-provider-8"),
            ])
        }
    });
    let transform: Rc<dyn VirtualLayoutSemanticCoordinateTransform> = Rc::new({
        let transform_calls = Rc::clone(&transform_calls);
        move |request: &radiant::runtime::virtual_layout::VirtualLayoutSemanticCoordinateTransformRequest| {
            transform_calls.set(transform_calls.get() + 1);
            let clip = request.destination_clip();
            VirtualLayoutSemanticCoordinateTransformOutcome::Found(Rect::from_min_max(
                Point::new(clip.max.x, clip.min.y),
                Point::new(clip.max.x + 1.0, clip.max.y),
            ))
        }
    });
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            let parts = public_parts(Some(Rc::clone(&provider)))
                .with_semantic_coordinate_transform(
                    VirtualLayoutPolicyIdentity::new("edge-touching-space"),
                    23,
                    Rc::clone(&transform),
                );
            arc_surface(virtual_layout_from_parts(parts).into_surface())
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    let publication = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("edge contact is represented by a conservative publication");

    assert_eq!(
        publication.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected,
        }
    );
    assert_eq!(provider_calls.get(), 1);
    assert_eq!(transform_calls.get(), 1);
    assert!(
        publication
            .selected
            .targets
            .targets
            .iter()
            .all(|target| !target.id.0.starts_with("edge-touching-provider-"))
    );
}

#[test]
fn zero_area_surface_intersection_invalidates_prior_custom_publication() {
    let provider_calls = Rc::new(Cell::new(0));
    let transform_calls = Rc::new(Cell::new(0));
    let provider = degenerate_clip_provider(Rc::clone(&provider_calls));
    let transform = degenerate_clip_transform(Rc::clone(&transform_calls));
    let degenerate = Rc::new(Cell::new(false));
    let bridge = declarative_runtime_bridge(
        (),
        {
            let degenerate = Rc::clone(&degenerate);
            let provider = Rc::clone(&provider);
            let transform = Rc::clone(&transform);
            move |_state: &mut ()| {
                let surface = if degenerate.get() {
                    custom_surface_with_surface_intersection(
                        true,
                        Rc::clone(&provider),
                        Rc::clone(&transform),
                    )
                } else {
                    custom_surface_with_surface_intersection(
                        false,
                        Rc::clone(&provider),
                        Rc::clone(&transform),
                    )
                };
                arc_surface(surface)
            }
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    let initial = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("the initial non-degenerate custom publication succeeds");
    assert_eq!(initial.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(provider_calls.get(), 1);
    assert_eq!(transform_calls.get(), 2);

    degenerate.set(true);
    runtime.refresh_with_scope(RepaintScope::Surface);
    let ordinary = runtime.automation_snapshot();
    let ordinary_targets = runtime.automation_target_snapshot();
    let publication = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("a zero-area surface intersection publishes the ordinary baseline");

    assert_eq!(
        publication.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected,
        }
    );
    assert_eq!(
        provider_calls.get(),
        1,
        "degenerate admission skips the provider"
    );
    assert_eq!(
        transform_calls.get(),
        2,
        "degenerate admission skips the coordinate transform"
    );
    assert_eq!(publication.selected.snapshot, ordinary);
    assert_eq!(publication.selected.targets, ordinary_targets);
    assert!(
        publication
            .selected
            .targets
            .targets
            .iter()
            .all(|target| !target.id.0.starts_with("degenerate-provider-"))
    );
    let selected = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("selected reads remain passive after invalidation");
    assert_eq!(selected.status, publication.status);
    assert_eq!(selected.snapshot, ordinary);
    assert_eq!(selected.targets, ordinary_targets);
}

#[test]
fn zero_area_own_viewport_intersection_publishes_ordinary_baseline_without_calls() {
    let provider_calls = Rc::new(Cell::new(0));
    let transform_calls = Rc::new(Cell::new(0));
    let bridge = declarative_runtime_bridge(
        (),
        {
            let provider = degenerate_clip_provider(Rc::clone(&provider_calls));
            let transform = degenerate_clip_transform(Rc::clone(&transform_calls));
            move |_state: &mut ()| {
                arc_surface(custom_surface_for_clip_kind(
                    DegenerateClipKind::OwnViewport,
                    Rc::clone(&provider),
                    Rc::clone(&transform),
                ))
            }
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    let ordinary = runtime.automation_snapshot();
    let ordinary_targets = runtime.automation_target_snapshot();
    let publication = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("a zero-area own viewport publishes the ordinary baseline");

    assert_eq!(
        publication.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected,
        }
    );
    assert_eq!(
        provider_calls.get(),
        0,
        "degenerate admission skips the provider"
    );
    assert_eq!(
        transform_calls.get(),
        0,
        "degenerate admission skips the coordinate transform"
    );
    assert_eq!(publication.selected.snapshot, ordinary);
    assert_eq!(publication.selected.targets, ordinary_targets);
    assert!(
        publication
            .selected
            .targets
            .targets
            .iter()
            .all(|target| !target.id.0.starts_with("degenerate-provider-"))
    );
}

#[test]
fn zero_area_ancestor_intersection_publishes_ordinary_baseline_without_calls() {
    let provider_calls = Rc::new(Cell::new(0));
    let transform_calls = Rc::new(Cell::new(0));
    let bridge = declarative_runtime_bridge(
        (),
        {
            let provider = degenerate_clip_provider(Rc::clone(&provider_calls));
            let transform = degenerate_clip_transform(Rc::clone(&transform_calls));
            move |_state: &mut ()| {
                arc_surface(custom_surface_for_clip_kind(
                    DegenerateClipKind::Ancestor,
                    Rc::clone(&provider),
                    Rc::clone(&transform),
                ))
            }
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    let ordinary = runtime.automation_snapshot();
    let ordinary_targets = runtime.automation_target_snapshot();
    let publication = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("a zero-area ancestor intersection publishes the ordinary baseline");

    assert_eq!(
        publication.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected,
        }
    );
    assert_eq!(
        provider_calls.get(),
        0,
        "degenerate admission skips the provider"
    );
    assert_eq!(
        transform_calls.get(),
        0,
        "degenerate admission skips the coordinate transform"
    );
    assert_eq!(publication.selected.snapshot, ordinary);
    assert_eq!(publication.selected.targets, ordinary_targets);
    assert!(
        publication
            .selected
            .targets
            .targets
            .iter()
            .all(|target| !target.id.0.starts_with("degenerate-provider-"))
    );
}

#[test]
fn direct_nested_public_shell_is_rejected_without_provider_authority() {
    let inner_calls = Rc::new(Cell::new(0));
    let provider_calls = Rc::clone(&inner_calls);
    let inner_provider: Rc<dyn VirtualLayoutSemanticRangeProvider> =
        Rc::new(move |_request: &VirtualLayoutSemanticRangeRequest| {
            provider_calls.set(provider_calls.get() + 1);
            VirtualLayoutSemanticProviderOutcome::Found(vec![semantic_entry(
                7,
                0,
                "nested-provider",
            )])
        });

    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            let inner_provider = Rc::clone(&inner_provider);
            let mut outer_parts =
                public_parts_with(2, VirtualLayoutRevisions::new(5, 6, 7, 8), None, None);
            outer_parts.shell = Rc::new(move || {
                virtual_layout_from_parts(public_parts_with(
                    3,
                    VirtualLayoutRevisions::new(9, 10, 11, 12),
                    None,
                    Some(Rc::clone(&inner_provider)),
                ))
            });
            arc_surface(virtual_layout_from_parts(outer_parts).into_surface())
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));

    assert_eq!(
        inner_calls.get(),
        0,
        "mounting must not call the inner provider"
    );
    let session = runtime
        .open_semantic_automation_session()
        .expect("session opens after rejecting the nested shell");
    let containers = runtime
        .semantic_automation_containers(session)
        .expect("container enumeration succeeds");
    assert!(
        containers.is_empty(),
        "a rejected nested shell must not install inner or outer provider authority"
    );
    assert_eq!(
        inner_calls.get(),
        0,
        "the rejected inner provider stays unused"
    );
}

#[test]
fn public_range_provider_is_explicit_only_and_preserves_unmaterialized_authority() {
    let calls = Rc::new(Cell::new(0));
    let provider_calls = Rc::clone(&calls);
    let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> =
        Rc::new(move |request: &VirtualLayoutSemanticRangeRequest| {
            provider_calls.set(provider_calls.get() + 1);
            assert_eq!(request.start_index(), 0);
            assert_eq!(request.length(), 2);
            assert_eq!(request.end_index(), 2);
            assert_eq!(request.revisions(), VirtualLayoutRevisions::new(1, 2, 3, 4));
            VirtualLayoutSemanticProviderOutcome::Found(vec![
                semantic_entry(7, 0, "provider-7"),
                semantic_entry(8, 1, "provider-8"),
            ])
        });

    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts(Some(Rc::clone(&provider)))).into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));

    assert_eq!(calls.get(), 0, "mounting must not call the provider");
    let session = runtime
        .open_semantic_automation_session()
        .expect("session opens");
    assert_eq!(calls.get(), 0, "opening must not call the provider");
    let containers = runtime
        .semantic_automation_containers(session)
        .expect("container enumeration succeeds");
    assert_eq!(containers.len(), 1);
    assert_eq!(calls.get(), 0, "enumeration must not call the provider");

    let ordinary = runtime.automation_snapshot();
    let ordinary_targets = runtime.automation_target_snapshot();
    runtime.refresh_with_scope(RepaintScope::PaintOnly);
    let selected_before_refresh = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("selected read succeeds");
    assert_eq!(
        calls.get(),
        0,
        "ordinary reads and repaint are provider-free"
    );
    assert_eq!(selected_before_refresh.snapshot, ordinary);
    assert_eq!(selected_before_refresh.targets, ordinary_targets);

    let refresh = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(containers[0], 0, 2)],
        )
        .expect("explicit range refresh succeeds");
    assert_eq!(calls.get(), 1, "one explicit range refresh makes one call");
    assert_eq!(refresh.status, SemanticAutomationRefreshStatus::Published);

    let selected = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("selected provider publication reads");
    assert_eq!(selected.status, SemanticAutomationRefreshStatus::Published);
    let first = selected
        .targets
        .targets
        .iter()
        .find(|target| target.id.0 == "provider-7")
        .expect("provider id survives the public path");
    let second = selected
        .targets
        .targets
        .iter()
        .find(|target| target.id.0 == "provider-8")
        .expect("unmaterialized provider id survives the public path");
    assert_eq!(first.label.as_deref(), Some("Provider row 7"));
    assert_eq!(first.bounds.x, 0.0);
    assert_eq!(first.bounds.y, 0.0);
    assert_eq!(second.label.as_deref(), Some("Provider row 8"));
    assert_eq!(second.bounds.y, 24.0);
    assert_eq!(
        second.authority,
        Some(radiant::gui::automation::AutomationTargetAuthority {
            runtime_generation: runtime.refresh_counters().runtime_projection,
            materialized: false,
        })
    );
    assert_eq!(
        calls.get(),
        1,
        "selected reads must not reenter the provider"
    );
}

#[test]
fn public_required_item_provider_is_explicit_only_and_does_not_call_range_provider() {
    let item_calls = Rc::new(Cell::new(0));
    let range_calls = Rc::new(Cell::new(0));
    let item_provider_calls = Rc::clone(&item_calls);
    let item_provider: Rc<dyn VirtualLayoutSemanticProvider> =
        Rc::new(move |request: &VirtualLayoutSemanticRequest| {
            item_provider_calls.set(item_provider_calls.get() + 1);
            assert_eq!(request.key(), &VirtualLayoutItemKey::new(7_u32));
            assert_eq!(request.revisions(), VirtualLayoutRevisions::new(1, 2, 3, 4));
            VirtualLayoutSemanticProviderOutcome::Found(semantic_entry(7, 0, "item-provider-7"))
        });
    let range_provider_calls = Rc::clone(&range_calls);
    let range_provider: Rc<dyn VirtualLayoutSemanticRangeProvider> =
        Rc::new(move |_request: &VirtualLayoutSemanticRangeRequest| {
            range_provider_calls.set(range_provider_calls.get() + 1);
            VirtualLayoutSemanticProviderOutcome::Rejected
        });
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts_with(
                    1,
                    VirtualLayoutRevisions::new(1, 2, 3, 4),
                    Some(Rc::clone(&item_provider)),
                    Some(Rc::clone(&range_provider)),
                ))
                .into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let session = runtime
        .open_semantic_automation_session()
        .expect("session opens");
    let containers = runtime
        .semantic_automation_containers(session)
        .expect("container enumeration succeeds");
    assert_eq!(item_calls.get(), 0);
    assert_eq!(range_calls.get(), 0);

    let refresh = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::required_item(
                containers[0],
                VirtualLayoutItemKey::new(7_u32),
            )],
        )
        .expect("explicit required-item refresh succeeds");
    assert_eq!(refresh.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(item_calls.get(), 1);
    assert_eq!(range_calls.get(), 0);
    assert!(
        refresh
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "item-provider-7")
    );
}

#[derive(Clone, Copy)]
enum RangeResponse {
    Found,
    DuplicateProviderId,
    NotFound,
    DataUnavailable,
    Deferred,
    Unsupported,
    Panic,
    Malformed,
}

fn controlled_range_provider(
    response: Rc<Cell<RangeResponse>>,
    calls: Rc<Cell<usize>>,
) -> Rc<dyn VirtualLayoutSemanticRangeProvider> {
    Rc::new(move |_request: &VirtualLayoutSemanticRangeRequest| {
        calls.set(calls.get() + 1);
        match response.get() {
            RangeResponse::Found => VirtualLayoutSemanticProviderOutcome::Found(vec![
                semantic_entry(7, 0, "controlled-7"),
                semantic_entry(8, 1, "controlled-8"),
            ]),
            RangeResponse::DuplicateProviderId => {
                VirtualLayoutSemanticProviderOutcome::Found(vec![
                    semantic_entry(7, 0, "duplicate-provider-id"),
                    semantic_entry(8, 1, "duplicate-provider-id"),
                ])
            }
            RangeResponse::NotFound => VirtualLayoutSemanticProviderOutcome::NotFound,
            RangeResponse::DataUnavailable => VirtualLayoutSemanticProviderOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::DataUnavailable,
            ),
            RangeResponse::Deferred => VirtualLayoutSemanticProviderOutcome::Deferred(
                VirtualLayoutSemanticDeferredReason::SemanticPending,
            ),
            RangeResponse::Unsupported => VirtualLayoutSemanticProviderOutcome::Unavailable(
                VirtualLayoutSemanticUnavailableReason::Unsupported,
            ),
            RangeResponse::Panic => panic!("public provider panic fixture"),
            RangeResponse::Malformed => VirtualLayoutSemanticProviderOutcome::Found(vec![
                semantic_entry(7, 1, "malformed-7"),
                semantic_entry(7, 2, "malformed-duplicate"),
            ]),
        }
    })
}

fn start_range_session<Bridge>(
    runtime: &mut SurfaceRuntime<Bridge, ()>,
) -> (
    radiant::runtime::SemanticAutomationSessionHandle,
    radiant::runtime::SemanticAutomationContainerHandle,
)
where
    Bridge: RuntimeBridge<()>,
{
    let session = runtime
        .open_semantic_automation_session()
        .expect("session opens");
    let containers = runtime
        .semantic_automation_containers(session)
        .expect("container enumeration succeeds");
    assert_eq!(containers.len(), 1);
    (session, containers[0])
}

#[test]
fn public_range_fallbacks_are_exact_and_panics_leave_runtime_usable() {
    let response = Rc::new(Cell::new(RangeResponse::Found));
    let calls = Rc::new(Cell::new(0));
    let provider = controlled_range_provider(Rc::clone(&response), Rc::clone(&calls));
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts(Some(Rc::clone(&provider)))).into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);

    let first = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("initial provider publication succeeds");
    assert_eq!(first.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(calls.get(), 1);

    response.set(RangeResponse::DataUnavailable);
    let retained = runtime
        .retry_semantic_automation_session(session)
        .expect("data-unavailable retry returns a selection");
    assert_eq!(
        retained.status,
        SemanticAutomationRefreshStatus::Retained {
            reason: SemanticAutomationFallbackReason::DataUnavailable,
        }
    );
    assert!(
        retained
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "controlled-8")
    );

    response.set(RangeResponse::Deferred);
    let deferred = runtime
        .retry_semantic_automation_session(session)
        .expect("deferred retry returns the exact prior selection");
    assert_eq!(
        deferred.status,
        SemanticAutomationRefreshStatus::Retained {
            reason: SemanticAutomationFallbackReason::Deferred,
        }
    );
    assert!(
        deferred
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "controlled-7")
    );

    response.set(RangeResponse::NotFound);
    let not_found = runtime
        .retry_semantic_automation_session(session)
        .expect("authoritative empty result is a complete publication");
    assert_eq!(not_found.status, SemanticAutomationRefreshStatus::Published);
    assert!(
        !not_found
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0.starts_with("controlled-"))
    );

    response.set(RangeResponse::Panic);
    let panicked = runtime
        .retry_semantic_automation_session(session)
        .expect("provider panic is contained at the public boundary");
    assert!(matches!(
        panicked.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected
        }
    ));
    assert!(
        !panicked
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "controlled-7")
    );
    assert_eq!(calls.get(), 5);

    response.set(RangeResponse::Found);
    let after_panic = runtime
        .retry_semantic_automation_session(session)
        .expect("runtime remains usable after provider panic");
    assert_eq!(
        after_panic.status,
        SemanticAutomationRefreshStatus::Published
    );
    assert_eq!(calls.get(), 6);
}

#[test]
fn public_malformed_and_terminal_results_publish_no_partial_selection() {
    let response = Rc::new(Cell::new(RangeResponse::Malformed));
    let calls = Rc::new(Cell::new(0));
    let provider = controlled_range_provider(Rc::clone(&response), Rc::clone(&calls));
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts(Some(Rc::clone(&provider)))).into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);

    let malformed = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("malformed provider output is classified, not panicked");
    assert!(matches!(
        malformed.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Malformed
                | SemanticAutomationFallbackReason::Rejected
        }
    ));
    assert!(
        !malformed
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0.starts_with("malformed-"))
    );

    response.set(RangeResponse::Unsupported);
    let terminal = runtime
        .refresh_semantic_automation_session(session, &[])
        .expect("clearing demand set succeeds");
    assert_eq!(
        terminal.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::NoDemand,
        }
    );
    let terminal = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("unsupported is a terminal provider result");
    assert_eq!(
        terminal.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Unsupported,
        }
    );
    assert!(
        !terminal
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0.starts_with("controlled-"))
    );
    assert_eq!(calls.get(), 2);
}

#[test]
fn public_duplicate_provider_ids_reject_whole_range_without_partial_targets() {
    let response = Rc::new(Cell::new(RangeResponse::DuplicateProviderId));
    let calls = Rc::new(Cell::new(0));
    let provider = controlled_range_provider(Rc::clone(&response), Rc::clone(&calls));
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts(Some(Rc::clone(&provider)))).into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);

    let publication = runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("duplicate provider IDs are classified, not panicked");
    assert_eq!(
        publication.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Malformed,
        }
    );
    assert!(
        !publication
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "duplicate-provider-id"),
        "a collision must not publish either range entry"
    );
    assert_eq!(calls.get(), 1);
}

#[test]
fn public_provider_reentry_is_rejected_atomically_and_runtime_remains_usable() {
    let calls = Rc::new(Cell::new(0));
    let reentry_attempts = Rc::new(Cell::new(0));
    let attempt_reentry = Rc::new(Cell::new(true));
    let runtime_slot: Rc<RefCell<Option<SurfaceRuntime<_, ()>>>> = Rc::new(RefCell::new(None));
    let session_slot = Rc::new(Cell::new(None));
    let container_slot = Rc::new(Cell::new(None));

    let provider_calls = Rc::clone(&calls);
    let provider_reentry_attempts = Rc::clone(&reentry_attempts);
    let provider_attempt_reentry = Rc::clone(&attempt_reentry);
    let provider_runtime = Rc::clone(&runtime_slot);
    let provider_session = Rc::clone(&session_slot);
    let provider_container = Rc::clone(&container_slot);
    let provider: Rc<dyn VirtualLayoutSemanticRangeProvider> =
        Rc::new(move |request: &VirtualLayoutSemanticRangeRequest| {
            provider_calls.set(provider_calls.get() + 1);
            if provider_attempt_reentry.replace(false) {
                provider_reentry_attempts.set(provider_reentry_attempts.get() + 1);
                let session = provider_session
                    .get()
                    .expect("session handle is installed before demand");
                let container = provider_container
                    .get()
                    .expect("container handle is installed before demand");

                // The shipped public callback is synchronous and receives no
                // runtime handle. Through an external holder, the only
                // reachable nested demand attempt observes the outer
                // `&mut SurfaceRuntime` borrow and is rejected before another
                // callback can start.
                let nested = provider_runtime.try_borrow_mut().map(|mut runtime| {
                    runtime
                        .as_mut()
                        .expect("runtime is installed")
                        .refresh_semantic_automation_session(
                            session,
                            &[SemanticAutomationDemand::range(
                                container,
                                request.start_index(),
                                request.length(),
                            )],
                        )
                });
                assert!(
                    nested.is_err(),
                    "a provider callback must not reenter the active runtime"
                );
                return VirtualLayoutSemanticProviderOutcome::Rejected;
            }
            VirtualLayoutSemanticProviderOutcome::Found(vec![
                semantic_entry(7, 0, "reentry-recovered-7"),
                semantic_entry(8, 1, "reentry-recovered-8"),
            ])
        });

    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            arc_surface(
                virtual_layout_from_parts(public_parts(Some(Rc::clone(&provider)))).into_surface(),
            )
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let (session, container) = start_range_session(&mut runtime);
    session_slot.set(Some(session));
    container_slot.set(Some(container));
    runtime_slot.borrow_mut().replace(runtime);

    let rejected = runtime_slot
        .borrow_mut()
        .as_mut()
        .expect("runtime is installed")
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(container, 0, 2)],
        )
        .expect("reentry is represented as a rejected provider result");
    assert_eq!(
        rejected.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Rejected,
        }
    );
    assert_eq!(calls.get(), 1, "reentry must not start a nested callback");
    assert_eq!(reentry_attempts.get(), 1);
    assert!(
        !rejected
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0.starts_with("reentry-recovered-")),
        "rejected reentry must not partially publish provider targets"
    );

    let recovered = runtime_slot
        .borrow_mut()
        .as_mut()
        .expect("runtime remains installed")
        .retry_semantic_automation_session(session)
        .expect("runtime remains usable after rejected reentry");
    assert_eq!(recovered.status, SemanticAutomationRefreshStatus::Published);
    assert_eq!(calls.get(), 2);
    assert!(
        recovered
            .selected
            .targets
            .targets
            .iter()
            .any(|target| target.id.0 == "reentry-recovered-7")
    );
}

struct DynamicSurfaceConfig {
    mounted: bool,
    policy_scope: u32,
    revisions: VirtualLayoutRevisions,
    semantic_cardinality: Option<VirtualLayoutSemanticCardinality>,
    item_provider: Option<Rc<dyn VirtualLayoutSemanticProvider>>,
    range_provider: Option<Rc<dyn VirtualLayoutSemanticRangeProvider>>,
}

#[test]
fn public_provider_replacement_scope_removal_and_close_fence_old_handles() {
    let first_calls = Rc::new(Cell::new(0));
    let second_calls = Rc::new(Cell::new(0));
    let first_provider = controlled_range_provider(
        Rc::new(Cell::new(RangeResponse::Found)),
        Rc::clone(&first_calls),
    );
    let second_provider = controlled_range_provider(
        Rc::new(Cell::new(RangeResponse::Found)),
        Rc::clone(&second_calls),
    );
    let config = Rc::new(RefCell::new(DynamicSurfaceConfig {
        mounted: true,
        policy_scope: 1,
        revisions: VirtualLayoutRevisions::new(1, 2, 3, 4),
        semantic_cardinality: Some(VirtualLayoutSemanticCardinality::new(0, 10)),
        item_provider: None,
        range_provider: Some(Rc::clone(&first_provider)),
    }));
    let bridge_config = Rc::clone(&config);
    let bridge = declarative_runtime_bridge(
        (),
        move |_state: &mut ()| {
            let (
                mounted,
                policy_scope,
                revisions,
                semantic_cardinality,
                item_provider,
                range_provider,
            ) = {
                let config = bridge_config.borrow();
                (
                    config.mounted,
                    config.policy_scope,
                    config.revisions,
                    config.semantic_cardinality,
                    config.item_provider.clone(),
                    config.range_provider.clone(),
                )
            };
            if mounted {
                let mut parts =
                    public_parts_with(policy_scope, revisions, item_provider, range_provider);
                parts.semantic_cardinality = semantic_cardinality;
                arc_surface(virtual_layout_from_parts(parts).into_surface())
            } else {
                arc_surface(ui::empty::<()>().into_surface())
            }
        },
        |_state: &mut (), _message: ()| {},
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 100.0));
    let session = runtime
        .open_semantic_automation_session()
        .expect("session opens");
    let old_container = runtime
        .semantic_automation_containers(session)
        .expect("old container enumerates")[0];
    runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(old_container, 0, 2)],
        )
        .expect("first provider publishes");
    assert_eq!(first_calls.get(), 1);

    config.borrow_mut().range_provider = Some(Rc::clone(&second_provider));
    runtime.refresh_with_scope(RepaintScope::Projection);
    assert_eq!(
        first_calls.get(),
        1,
        "replacement does not call the new provider"
    );
    assert_eq!(second_calls.get(), 0);
    let replaced = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("selection read after replacement");
    assert!(matches!(
        replaced.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Invalidated
        }
    ));
    // Public providers are synchronous, so an actual late return cannot be
    // held across this replacement. The private exact-fence test covers that
    // unrepresentable transport path; this public replacement proves the old
    // authority is inert before a new explicit refresh.
    assert!(
        !replaced
            .targets
            .targets
            .iter()
            .any(|target| target.id.0.starts_with("controlled-"))
    );
    runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(old_container, 0, 2)],
        )
        .expect("same-scope handle remains valid after provider replacement");
    assert_eq!(second_calls.get(), 1);

    config.borrow_mut().semantic_cardinality =
        Some(VirtualLayoutSemanticCardinality::new(usize::MAX, 10));
    runtime.refresh_with_scope(RepaintScope::Projection);
    let count_invalidated = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("count-change selection read");
    assert!(matches!(
        count_invalidated.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Invalidated
        }
    ));
    assert_eq!(
        second_calls.get(),
        1,
        "count synchronization is provider-free"
    );
    runtime
        .refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(old_container, 0, 2)],
        )
        .expect("the unchanged provider can republish after count invalidation");
    assert_eq!(second_calls.get(), 2);

    config.borrow_mut().semantic_cardinality = Some(VirtualLayoutSemanticCardinality::new(3, 11));
    runtime.refresh_with_scope(RepaintScope::Projection);
    let revision_invalidated = runtime
        .selected_semantic_automation_snapshot(session)
        .expect("revision-change selection read");
    assert!(matches!(
        revision_invalidated.status,
        SemanticAutomationRefreshStatus::Baseline {
            reason: SemanticAutomationFallbackReason::Invalidated
        }
    ));
    assert_eq!(
        second_calls.get(),
        2,
        "cardinality revision synchronization is provider-free"
    );

    config.borrow_mut().policy_scope = 2;
    runtime.refresh_with_scope(RepaintScope::Projection);
    let stale_scope = runtime.refresh_semantic_automation_session(
        session,
        &[SemanticAutomationDemand::range(old_container, 0, 2)],
    );
    assert_eq!(
        stale_scope,
        Err(SemanticAutomationSessionError::StaleContainerHandle)
    );
    let new_container = runtime
        .semantic_automation_containers(session)
        .expect("new scope enumerates")[0];
    assert_ne!(old_container, new_container);

    config.borrow_mut().mounted = false;
    runtime.refresh_with_scope(RepaintScope::Projection);
    assert!(
        runtime
            .semantic_automation_containers(session)
            .expect("session remains open while container is removed")
            .is_empty()
    );
    assert_eq!(
        runtime.refresh_semantic_automation_session(
            session,
            &[SemanticAutomationDemand::range(new_container, 0, 2)],
        ),
        Err(SemanticAutomationSessionError::StaleContainerHandle)
    );
    runtime
        .close_semantic_automation_session(session)
        .expect("session closes");
    assert_eq!(
        runtime.selected_semantic_automation_snapshot(session),
        Err(SemanticAutomationSessionError::UnknownSession)
    );
    assert_eq!(
        second_calls.get(),
        2,
        "subsequent lifecycle fences do not call providers"
    );
}
