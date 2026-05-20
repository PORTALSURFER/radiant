use super::super::*;

#[test]
fn native_gpu_hover_survives_scene_rebuilds() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.last_cursor = Some(Point::new(60.0, 20.0));
    runner.rebuild_scene();

    let surface = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    assert!(surface.overlays.iter().any(|overlay| matches!(
        overlay,
        GpuSurfaceOverlay::RuntimeVerticalLine { ratio, .. } if (*ratio - 0.25).abs() < 0.001
    )));

    runner.rebuild_scene();
    let surface = runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    assert!(surface.overlays.iter().any(|overlay| matches!(
        overlay,
        GpuSurfaceOverlay::RuntimeVerticalLine { ratio, .. } if (*ratio - 0.25).abs() < 0.001
    )));
}
