use super::super::*;

#[test]
fn native_gpu_hover_preserves_app_owned_vertical_overlays() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let surface = runner
        .frame
        .last_paint_plan
        .primitives
        .iter_mut()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    surface.overlays.push(GpuSurfaceOverlay::VerticalCursor {
        ratio: 0.5,
        color: Rgba8 {
            r: 0,
            g: 220,
            b: 255,
            a: 255,
        },
        width: 2.0,
    });

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    assert!(runner.clear_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
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
        GpuSurfaceOverlay::VerticalCursor { ratio, .. } if (*ratio - 0.5).abs() < 0.001
    )));
    assert!(
        !surface
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }))
    );
}

#[test]
fn native_gpu_hover_clear_hides_cached_cursor_without_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    assert!(runner.clear_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
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
    assert!(
        surface
            .capabilities
            .runtime_overlays
            .pointer_vertical_line
            .is_some()
    );
    assert!(
        !surface
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }))
    );
}
