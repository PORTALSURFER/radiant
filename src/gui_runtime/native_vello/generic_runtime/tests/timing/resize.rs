use super::super::super::runner_state::{SurfaceAcquirePolicy, surface_acquire_policy};
use super::{fixtures::*, shared::*};
use vello::wgpu;

#[test]
fn deferred_surface_resize_keeps_latest_nonzero_size() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_surface_resize(PhysicalSize::new(400, 240));
    runner.defer_surface_resize(PhysicalSize::new(0, 480));
    runner.defer_surface_resize(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
    assert_eq!(
        runner.timing.pending_surface_resize_reason,
        Some(FrameWorkReason::NativeResize)
    );
}

#[test]
fn surface_acquire_policy_distinguishes_recovery_and_fence_states() {
    let nonzero = PhysicalSize::new(640, 360);

    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Lost, nonzero),
        SurfaceAcquirePolicy::ReconfigureAndRetry
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Outdated, nonzero),
        SurfaceAcquirePolicy::ReconfigureAndRetry
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Lost, PhysicalSize::new(0, 360)),
        SurfaceAcquirePolicy::Defer
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Outdated, PhysicalSize::new(640, 0)),
        SurfaceAcquirePolicy::Defer
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::OutOfMemory, nonzero),
        SurfaceAcquirePolicy::Terminal
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Timeout, nonzero),
        SurfaceAcquirePolicy::Timeout
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Other, nonzero),
        SurfaceAcquirePolicy::ConservativeFence
    );
    assert_eq!(
        surface_acquire_policy(wgpu::SurfaceError::Other, PhysicalSize::new(0, 360)),
        SurfaceAcquirePolicy::ConservativeFence
    );
}

#[test]
fn unbound_resources_preserve_pending_resize_without_native_work_or_retry() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let adapter = GenericNativeAdapterOwner::with_test_registration(
        NativeAdapterGeneration::from_test_serial(1),
        Arc::new(DeviceLossRegistration::new()),
    );

    runner.defer_surface_resize(PhysicalSize::new(640, 360));
    let initial_context_generation = runner.frame.native_scene_context_generation_for_test();
    runner.apply_pending_surface_resize_if_needed(&adapter);

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
    assert!(!runner.timing.redraw_requested);
    assert!(runner.window.native_surface_target_fenced);
    assert_eq!(
        runner.frame.native_scene_context_generation_for_test(),
        initial_context_generation + 1
    );
    assert!(!runner.admit_native_resources(&adapter));
    assert_eq!(
        runner.frame.native_scene_context_generation_for_test(),
        initial_context_generation + 1
    );
}

#[test]
fn other_failure_fences_native_target_without_reconfiguring_zero_size_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;
    let pending = FrameWork::RebuildScene {
        reason: FrameWorkReason::RuntimeSurfaceRepaint,
        mode: SceneRebuildMode::Immediate,
    };
    runner.timing.pending_frame_work = pending;

    runner.handle_other_surface_acquire_failure(PhysicalSize::new(0, 360));

    assert!(!runner.window.target_generation.is_known());
    assert!(runner.frame.scene_texture_dirty);
    assert!(runner.frame.composited_base_dirty);
    assert_eq!(runner.timing.pending_frame_work, pending);
    assert_eq!(runner.timing.pending_surface_resize, None);
    assert_eq!(
        runner
            .window
            .surface_recovery
            .diagnostics()
            .completed_reconfigures,
        0
    );
    assert!(runner.window.native_surface_target_fenced);
    assert_eq!(runner.window.surface_recovery.diagnostics().others, 1);
    assert_eq!(
        runner
            .window
            .surface_recovery
            .diagnostics()
            .other_retry_requests,
        0
    );
}

#[test]
fn repeated_target_fences_are_idempotent() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());

    runner.handle_other_surface_acquire_failure(PhysicalSize::new(640, 360));
    let context_generation = runner.frame.native_scene_context_generation_for_test();
    runner.handle_other_surface_acquire_failure(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.frame.native_scene_context_generation_for_test(),
        context_generation
    );
    assert!(!runner.window.target_generation.is_known());
    assert!(runner.window.native_surface_target_fenced);
}

#[test]
fn successful_acquisition_after_other_fence_promotes_fresh_target_generation() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    assert!(runner.window.target_generation.advance());
    let previous = runner.window.target_generation;

    runner.handle_other_surface_acquire_failure(PhysicalSize::new(640, 360));
    assert!(!runner.window.target_generation.is_known());
    runner.prepare_successful_surface_acquisition();

    assert!(runner.window.target_generation.is_known());
    assert_ne!(runner.window.target_generation, previous);
    assert!(runner.frame.scene_texture_dirty);
    assert!(runner.frame.composited_base_dirty);
    let promoted = runner.window.target_generation;
    runner.prepare_successful_surface_acquisition();
    assert_eq!(runner.window.target_generation, promoted);
}

#[test]
fn native_resize_event_waits_for_confirmed_resize_before_reporting_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.resize_surface(PhysicalSize::new(400, 240));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(400, 240))
    );
    assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
}

#[test]
fn command_resize_reason_survives_deferred_surface_resize() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_surface_resize_with_reason(
        PhysicalSize::new(480, 270),
        FrameWorkReason::CommandResize,
    );

    assert_eq!(
        runner.timing.pending_surface_resize_reason,
        Some(FrameWorkReason::CommandResize)
    );
}

#[test]
fn window_resize_events_coalesce_until_redraw_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.resize_surface(PhysicalSize::new(400, 240));
    runner.resize_surface(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
}

#[test]
fn simple_dirty_resize_frame_can_render_directly_to_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.timing.surface_resize_applied_this_frame = true;
    runner.frame.scene_texture_dirty = true;

    assert!(runner.should_render_resize_frame_directly());

    runner
        .frame
        .transient_overlay_primitives
        .push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }));

    assert!(!runner.should_render_resize_frame_directly());
}

#[test]
fn deferred_interactive_scene_rebuild_is_flushed_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(runner.frame.scene_texture_dirty);
}

#[test]
fn deferred_scene_rebuild_marks_pending_without_surface_refresh() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_scene_rebuild();

    assert!(runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
}

#[test]
fn deferred_viewport_resize_is_applied_at_scene_rebuild_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.record_frame_work(FrameWork::ResizeSurface {
        reason: FrameWorkReason::CommandResize,
    });
    runner.defer_viewport_resize_with_reason(
        Vector2::new(640.0, 120.0),
        FrameWorkReason::CommandResize,
    );

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
    assert_eq!(
        runner.timing.pending_viewport_resize,
        Some(Vector2::new(640.0, 120.0))
    );

    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(640.0, 120.0));
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::ResizeAndRebuild {
            reason: FrameWorkReason::CommandResize,
        },
        "logical relayout should upgrade physical resize work to resize-and-rebuild"
    );
}

#[test]
fn subpixel_equivalent_resize_updates_viewport_without_relayout() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    assert!(!runner.core.set_viewport(Vector2::new(320.4, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));

    assert!(runner.core.set_viewport(Vector2::new(320.6, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.6, 40.0));
}

#[test]
fn subpixel_equivalent_deferred_resize_reuses_encoded_scene() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;

    runner.record_frame_work(FrameWork::ResizeSurface {
        reason: FrameWorkReason::NativeResize,
    });
    runner.defer_viewport_resize(Vector2::new(320.4, 40.0));
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));
    assert!(
        runner.frame.scene_texture_dirty,
        "the resized surface still needs a fresh texture render"
    );
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::ResizeSurface {
            reason: FrameWorkReason::NativeResize,
        },
        "subpixel-equivalent resize should not claim a scene rebuild"
    );
}

#[test]
fn deferred_refresh_with_subpixel_resize_reports_resize_and_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let project_count = runner.core.runtime.bridge().project_count;
    runner.timing.deferred_surface_refresh = true;
    runner.record_frame_work(FrameWork::RefreshSurface {
        reason: FrameWorkReason::DeferredSurfaceRefresh,
    });
    runner.record_frame_work(FrameWork::ResizeSurface {
        reason: FrameWorkReason::NativeResize,
    });
    runner.defer_viewport_resize(Vector2::new(320.4, 40.0));

    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1
    );
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::ResizeAndRebuild {
            reason: FrameWorkReason::NativeResize,
        },
        "surface refresh plus resize must report the scene rebuild performed by frame preparation"
    );
}

#[test]
fn deferred_auxiliary_sync_tracks_interactive_rebuild_deferral() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_auxiliary_window_sync();

    assert!(runner.timing.deferred_auxiliary_window_sync);
}

#[test]
fn deferred_interactive_scene_rebuild_refreshes_surface_once_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let project_count = runner.core.runtime.bridge().project_count;

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1,
        "deferred interactive rebuild should refresh and encode in one frame-boundary pass"
    );
}
