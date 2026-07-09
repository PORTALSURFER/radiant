use super::super::*;

#[test]
fn gpu_surface_fast_path_does_not_capture_horizontal_pan() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(runner.can_fast_path_gpu_surface_route(point, Vector2::new(0.0, -40.0)));
    assert!(!runner.can_fast_path_gpu_surface_route(point, Vector2::new(40.0, 1.0)));
}

#[test]
fn gpu_surface_wheel_fast_path_reports_deferred_refresh_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);
    let delta = Vector2::new(0.0, -40.0);
    let mut outcome = GenericRouteOutcome::default();
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);

    assert_eq!(
        outcome.frame_work(),
        FrameWork::RebuildScene {
            reason: FrameWorkReason::RoutedInput,
            mode: SceneRebuildMode::Immediate,
        },
        "generic routing should expose the rebuild work that the GPU fast path replaces"
    );

    runner.handle_gpu_surface_route_outcome(outcome, point, delta);

    assert!(runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RefreshSurface {
            reason: FrameWorkReason::DeferredSurfaceRefresh,
        },
        "presentation diagnostics should describe the deferred GPU refresh actually performed"
    );
}
