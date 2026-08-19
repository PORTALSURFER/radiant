use super::{fixtures::*, shared::*};
use crate::application::IntoView;
use std::{cell::Cell, rc::Rc};

struct ReadyVirtualLayoutPolicy {
    query_count: Rc<Cell<usize>>,
}

impl crate::layout::VirtualLayoutPolicy for ReadyVirtualLayoutPolicy {
    fn query(
        &self,
        _input: &crate::layout::VirtualLayoutQueryInput,
        sink: &mut crate::layout::VirtualLayoutQuerySink,
    ) -> crate::layout::VirtualLayoutPolicyDecision {
        self.query_count
            .set(self.query_count.get().saturating_add(1));
        sink.visit(crate::layout::VirtualLayoutItemCandidate::new(
            crate::layout::VirtualLayoutItemKey::new(1),
            0,
            crate::gui::types::Rect::from_xy_size(0.0, 0.0, 100.0, 20.0),
            crate::layout::VirtualLayoutVisibility::Visible,
            crate::layout::VirtualLayoutBoundsConfidence::Exact,
        ))
        .expect("the test virtual-layout budget admits one item");
        sink.set_extent(crate::layout::VirtualLayoutExtentCandidate::exact(
            Vector2::new(100.0, 20.0),
        ))
        .expect("the test virtual-layout policy supplies one extent");
        crate::layout::VirtualLayoutPolicyDecision::Ready
    }
}

struct ReadyVirtualLayoutBridge {
    policy_queries: Rc<Cell<usize>>,
    project_count: usize,
}

impl ReadyVirtualLayoutBridge {
    fn new(policy_queries: Rc<Cell<usize>>) -> Self {
        Self {
            policy_queries,
            project_count: 0,
        }
    }
}

impl crate::runtime::RuntimeBridge<()> for ReadyVirtualLayoutBridge {
    fn project_surface(&mut self) -> std::sync::Arc<crate::runtime::UiSurface<()>> {
        self.project_count += 1;
        let view = crate::application::virtual_layout::virtual_layout_from_parts(
            crate::application::virtual_layout::VirtualLayoutParts::new(
                Rc::new(ReadyVirtualLayoutPolicy {
                    query_count: Rc::clone(&self.policy_queries),
                }),
                crate::layout::VirtualLayoutPolicyIdentity::new("timing-test"),
                crate::layout::VirtualLayoutOverscan::new(0.0, 0.0)
                    .expect("valid test virtual-layout overscan"),
                crate::layout::VirtualLayoutBudget::new(1),
                crate::runtime::VirtualLayoutRevisions::default(),
                Rc::new(|| crate::application::column(std::iter::empty())),
                Rc::new(|_| crate::application::text::<()>("item")),
                Rc::new(|_| crate::layout::VirtualLayoutPolicyIdentity::new("item")),
            ),
        );
        crate::runtime::test_arc_surface(view.into_surface())
    }

    fn update(&mut self, _message: ()) -> crate::runtime::Command<()> {
        crate::runtime::Command::none()
    }
}

fn valid_prepared_surface_refresh_native_evidence() -> PreparedSurfaceRefreshNativeEvidence {
    PreparedSurfaceRefreshNativeEvidence {
        window_id: Some(winit::window::WindowId::dummy()),
        adapter_generation: Some(NativeAdapterGeneration::from_test_serial(1)),
        target_generation:
            super::super::super::runner_state::NativeTargetGeneration::from_test_serial(1),
        environment: crate::runtime::WindowEnvironment::default(),
        native_resources_present: true,
        target_fenced: false,
        pending_viewport_resize: false,
        pending_surface_resize: false,
        lifecycle: NativeLifecycle::default(),
        newer_visual_request: false,
    }
}

#[test]
fn transient_overlay_hint_skips_empty_app_overlay_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
}

#[test]
fn empty_overlay_paint_skips_app_and_runtime_overlay_callbacks() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
    assert!(runner.frame.transient_overlay_primitives.is_empty());
}

#[test]
fn explicit_transient_overlay_capability_runs_custom_bridge_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        OptInTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 1);
}

#[test]
fn exact_scene_refresh_reuses_encoded_scene_and_preserves_derived_state() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    let initial_stats = runner.frame.last_scene_stats;
    runner.frame.post_gpu_overlay_suffix_start = Some(7);
    runner.frame.post_gpu_overlay_has_replayable_suffix = true;
    runner.frame.scene_texture_dirty = false;

    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();
    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 1);
    assert_eq!(runner.frame.last_scene_stats, initial_stats);
    assert_eq!(runner.frame.post_gpu_overlay_suffix_start, Some(7));
    assert!(runner.frame.post_gpu_overlay_has_replayable_suffix);
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(runner.core.runtime.bridge().paint_calls, 1);
}

#[test]
fn ordinary_retained_surface_rebuild_does_not_clone_populated_cache() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        RetainedSurfaceBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    assert_eq!(runner.core.runtime.bridge().render_count, 1);
    assert_eq!(runner.frame.retained_surface_cache.entry_count(), 1);
    assert_eq!(runner.frame.last_scene_stats.cache_hits, 0);

    RetainedSurfaceFrameCache::reset_test_clone_count();
    runner.rebuild_scene();

    assert_eq!(runner.core.runtime.bridge().render_count, 1);
    assert_eq!(runner.frame.last_scene_stats.cache_hits, 1);
    assert_eq!(RetainedSurfaceFrameCache::test_clone_count(), 0);
}

#[test]
fn prepared_plan_admission_encodes_once_without_a_second_plan_build() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());

    runner.rebuild_scene();
    let before_refresh = runner.core.runtime.refresh_counters();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("prepared refresh candidate");
    let terminal_messages = runner
        .core
        .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared);
    assert!(terminal_messages.is_some());
    let after_plan = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_plan.base_paint_plan_rebuilds,
        before_refresh.base_paint_plan_rebuilds + 1
    );
    runner.frame.scene_texture_dirty = false;

    runner.complete_prepared_surface_refresh(terminal_messages.unwrap());

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .base_paint_plan_rebuilds,
        after_plan.base_paint_plan_rebuilds
    );
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(
        runner.frame.test_phase_trace(),
        [
            Some(super::super::super::frame_state::NativeVelloTestPhase::EligibilityObserved),
            Some(super::super::super::frame_state::NativeVelloTestPhase::SceneEncode),
        ]
    );
}

#[test]
fn prepared_refresh_veto_keeps_the_combined_refresh_fallback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let project_count = runner.core.runtime.bridge().project_count;
    runner.timing.deferred_surface_refresh = true;

    // Startup has no native adapter/window/resource evidence, so Projection
    // admission must veto before the prepared transaction and use the
    // existing combined refresh path.
    runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn stale_native_evidence_drops_held_candidate_without_publication_or_replay() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    let native_evidence = valid_prepared_surface_refresh_native_evidence();
    let mut stale_native_evidence = native_evidence;
    stale_native_evidence.target_generation =
        super::super::super::runner_state::NativeTargetGeneration::from_test_serial(2);
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test_with_current_evidence(
        &mut RenderFrameProfile::default(),
        native_evidence,
        stale_native_evidence,
    );

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1,
        "candidate preparation pulls once and must not replay combined projection"
    );
    let after_refresh = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_refresh.application_projection, before_refresh.application_projection,
        "stale native evidence must not publish the candidate transaction"
    );
    assert_eq!(
        after_refresh.runtime_projection,
        before_refresh.runtime_projection,
    );
    assert_eq!(
        after_refresh.widget_state_sync,
        before_refresh.widget_state_sync
    );
    assert_eq!(after_refresh.layout, before_refresh.layout);
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn active_virtual_layout_vetoes_prepared_admission_and_materializes_combined_refresh() {
    let policy_queries = Rc::new(Cell::new(0));
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ReadyVirtualLayoutBridge::new(Rc::clone(&policy_queries)),
        Vector2::new(240.0, 80.0),
    );
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();
    let before_project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    assert!(!runner.core.runtime.prepared_surface_refresh_is_eligible());
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    let after_refresh = runner.core.runtime.refresh_counters();
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        before_project_count + 1,
        "active virtual content must use the combined projection refresh"
    );
    assert_eq!(
        after_refresh.application_projection,
        before_refresh.application_projection + 1,
    );
    assert!(
        after_refresh.runtime_projection > before_refresh.runtime_projection,
        "combined virtual refresh must perform the runtime projection"
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn admitted_gpu_candidate_does_not_replay_combined_projection() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        UnsupportedPreparedRefreshBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    let project_count = runner.core.runtime.bridge().project_count;
    let before_refresh = runner.core.runtime.refresh_counters();
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    // Candidate preparation pulls once. A post-admission fallback would pull
    // and project a second time through the combined refresh path.
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1
    );
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .application_projection,
        before_refresh.application_projection + 1
    );
    assert!(
        runner
            .frame
            .last_paint_plan
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn prepared_refresh_dispatches_replacement_terminal_after_scene_admission() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner.frame.set_test_scene_encode_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_encode(&recorder)
    }));
    runner.frame.set_test_scene_admission_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_admission(&recorder)
    }));
    let before_refresh = runner.core.runtime.refresh_counters();
    runner.core.runtime.bridge_mut().replace = true;
    // The native owner needs a real window/device resource bundle; exercise
    // the same prepared transaction directly and keep completion on the
    // production ordering helper below.
    let prepared = runner
        .core
        .prepare_prepared_surface_refresh(crate::runtime::RepaintScope::Projection)
        .expect("prepared refresh candidate");
    let terminal_messages = runner
        .core
        .publish_prepared_surface_refresh(&mut runner.frame.last_paint_plan, prepared);
    let terminal_messages = terminal_messages.expect("prepared replacement terminal messages");
    assert_eq!(terminal_messages.len(), 1);

    let after_plan = runner.core.runtime.refresh_counters();
    assert_eq!(
        after_plan.base_paint_plan_rebuilds,
        before_refresh.base_paint_plan_rebuilds + 1
    );
    runner.frame.scene_texture_dirty = false;
    runner.complete_prepared_surface_refresh(terminal_messages);

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
    assert_eq!(
        prepared_refresh_events(&recorder),
        vec![
            PreparedRefreshEvent::SceneEncode,
            PreparedRefreshEvent::SceneAdmitted,
            PreparedRefreshEvent::TerminalUpdate(PreparedRefreshTerminalMessage),
        ]
    );
    assert!(runner.frame.scene_texture_dirty);
    assert_eq!(
        runner
            .core
            .runtime
            .refresh_counters()
            .base_paint_plan_rebuilds,
        after_plan.base_paint_plan_rebuilds
    );
}

#[test]
fn prepared_refresh_orders_projection_candidate_layout_publication_scene_and_terminal() {
    let recorder = prepared_refresh_scene_admission_recorder();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PreparedRefreshReplacementBridge::new(Rc::clone(&recorder)),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .set_test_prepared_surface_refresh_phase_observer(Rc::new({
            let recorder = Rc::clone(&recorder);
            move |phase| {
                let event = match phase {
                    "projection-admitted" => PreparedRefreshEvent::ProjectionAdmitted,
                    "candidate-held" => PreparedRefreshEvent::CandidateHeld,
                    "projection-complete" => PreparedRefreshEvent::ProjectionCompleted,
                    "layout-admitted" => PreparedRefreshEvent::LayoutAdmitted,
                    "published" => PreparedRefreshEvent::Published,
                    _ => panic!("unexpected prepared refresh phase: {phase}"),
                };
                recorder.borrow_mut().push(event);
            }
        }));
    runner.frame.set_test_scene_encode_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_encode(&recorder)
    }));
    runner.frame.set_test_scene_admission_observer(Rc::new({
        let recorder = Rc::clone(&recorder);
        move || record_prepared_refresh_scene_admission(&recorder)
    }));
    runner.core.runtime.bridge_mut().replace = true;
    runner.timing.deferred_surface_refresh = true;

    runner.refresh_deferred_surface_if_needed_for_test(
        &mut RenderFrameProfile::default(),
        valid_prepared_surface_refresh_native_evidence(),
    );

    assert_eq!(
        prepared_refresh_events(&recorder),
        vec![
            PreparedRefreshEvent::ProjectionAdmitted,
            PreparedRefreshEvent::CandidateHeld,
            PreparedRefreshEvent::ProjectionCompleted,
            PreparedRefreshEvent::LayoutAdmitted,
            PreparedRefreshEvent::Published,
            PreparedRefreshEvent::SceneEncode,
            PreparedRefreshEvent::SceneAdmitted,
            PreparedRefreshEvent::TerminalUpdate(PreparedRefreshTerminalMessage),
        ]
    );
    assert!(!runner.frame_stage_owner.has_in_flight());
}

#[test]
fn eligibility_observation_precedes_encode_without_changing_scene_counters() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());

    runner.rebuild_scene();
    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 0);

    runner.rebuild_scene();
    assert_eq!(
        runner.frame.last_native_paint_segment_eligibility.outcome,
        super::super::super::retained_paint_segments::NativePaintSegmentEligibilityOutcome::FullSceneFallback(
            super::super::super::retained_paint_segments::NativePaintSegmentFallbackReason::PaintConservative,
        )
    );
    assert_eq!(
        runner.frame.last_native_paint_segment_eligibility.entries,
        [None; crate::runtime::MAX_PAINT_SEGMENTS]
    );
    assert_eq!(
        runner
            .frame
            .last_native_paint_segment_eligibility
            .entry_count,
        0
    );
    assert_eq!(
        runner.frame.test_phase_trace(),
        [
            Some(super::super::super::frame_state::NativeVelloTestPhase::EligibilityObserved),
            Some(super::super::super::frame_state::NativeVelloTestPhase::SceneEncode),
        ]
    );
    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn environment_change_vetoes_exact_scene_reuse_and_reencodes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        OptInTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .runtime
        .set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            Some(crate::runtime::WindowColorScheme::Light),
            false,
            false,
        ));
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn exact_3k_runner_refresh_cohort_reuses_scene_encoding() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        LargeExactBridge,
        Vector2::new(960.0, 720.0),
    );

    runner.rebuild_scene();
    for _ in 0..3 {
        runner
            .core
            .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
        runner.rebuild_scene_after_surface_refresh();
    }

    assert_eq!(runner.frame.scene_encode_count, 1);
    assert_eq!(runner.frame.scene_reuse_count, 3);
    assert!(runner.frame.last_scene_stats.paint_plan_primitives > 0);
}

#[test]
fn invalidated_native_target_vetoes_exact_scene_reuse() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner.frame.invalidate_native_scene_context();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 0);
}

#[test]
fn standalone_scene_rebuild_without_exact_refresh_reencodes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ExactTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.rebuild_scene();
    runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Projection);
    runner.rebuild_scene_after_surface_refresh();
    runner.rebuild_scene();

    assert_eq!(runner.frame.scene_encode_count, 2);
    assert_eq!(runner.frame.scene_reuse_count, 1);
}

#[test]
fn minimal_bridge_skips_frame_diagnostics_callback_work() {
    let core = GenericNativeRuntimeCore::new(NoFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(!core.has_frame_diagnostics_observer());
}

#[test]
fn explicit_frame_diagnostics_capability_enables_callback_work() {
    let core =
        GenericNativeRuntimeCore::new(OptInFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(core.has_frame_diagnostics_observer());
}
