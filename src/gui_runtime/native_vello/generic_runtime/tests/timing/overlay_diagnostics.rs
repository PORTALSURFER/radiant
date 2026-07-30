use super::{fixtures::*, shared::*};

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
