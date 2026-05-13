use super::*;

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
fn gpu_surface_pointer_move_fast_path_only_within_cached_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();

    assert!(runner.can_fast_path_gpu_surface_pointer_move(
        Some(Point::new(20.0, 20.0)),
        Point::new(40.0, 20.0)
    ));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(None, Point::new(40.0, 20.0)));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(
        Some(Point::new(-4.0, 20.0)),
        Point::new(40.0, 20.0)
    ));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(
        Some(Point::new(20.0, 20.0)),
        Point::new(20.0, 90.0)
    ));
}

#[test]
fn gpu_surface_pointer_move_fast_path_is_disabled_during_pointer_capture() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(
        runner
            .core
            .route_pointer_press(point, PointerButton::Primary)
            .needs_redraw()
    );
    assert!(runner.core.runtime.pointer_capture().is_some());
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(Some(point), Point::new(40.0, 20.0)));
}

#[test]
fn native_gpu_hover_fast_path_is_disabled_during_pointer_capture() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(320.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(20.0, 20.0);

    assert!(runner.can_fast_path_native_hover_move(point));
    assert!(
        runner
            .core
            .route_pointer_press(point, PointerButton::Primary)
            .needs_redraw()
    );
    assert!(runner.core.runtime.pointer_capture().is_some());
    assert!(!runner.can_fast_path_native_hover_move(Point::new(40.0, 20.0)));
}

#[test]
fn plain_gpu_surface_does_not_opt_into_runtime_fast_paths() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge {
            capabilities: GpuSurfaceCapabilities::default(),
            ..GpuWheelBridge::default()
        },
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);

    assert!(!runner.can_fast_path_gpu_surface_route(point, Vector2::new(0.0, -40.0)));
    assert!(!runner.can_coalesce_gpu_surface_wheel(point, Vector2::new(0.0, -40.0)));
    assert!(!runner.can_fast_path_gpu_surface_pointer_move(Some(point), Point::new(80.0, 20.0)));
    assert!(!runner.update_gpu_surface_cursor_overlay(point));
}
