use super::{fixtures::*, shared::*};
use std::rc::Rc;

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
    let terminal_messages = runner.core.try_prepared_surface_refresh(
        crate::runtime::RepaintScope::Projection,
        &mut runner.frame.last_paint_plan,
        || true,
    );
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
    let terminal_messages = runner.core.try_prepared_surface_refresh(
        crate::runtime::RepaintScope::Projection,
        &mut runner.frame.last_paint_plan,
        || true,
    );
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
