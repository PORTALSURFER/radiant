use super::super::*;

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
