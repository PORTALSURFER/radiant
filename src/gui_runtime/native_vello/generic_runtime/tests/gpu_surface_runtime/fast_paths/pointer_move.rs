use super::super::*;

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
