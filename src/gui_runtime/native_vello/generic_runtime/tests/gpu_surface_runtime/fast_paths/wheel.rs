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
