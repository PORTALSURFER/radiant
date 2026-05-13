use super::*;

#[test]
fn native_gpu_hover_updates_cached_overlay_without_refreshing_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    runner.composited_base_dirty = false;
    let project_count = runner.core.runtime.bridge().project_count;

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    assert!(
        runner.composited_base_dirty,
        "cached composed frames must refresh when runtime GPU overlays change"
    );
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count,
        "native cursor updates should not refresh or reproject the app surface"
    );
    let surface = runner
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

#[test]
fn native_gpu_hover_skips_unchanged_cached_overlay() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    assert!(!runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(80.0, 20.0)));
}

#[test]
fn native_gpu_hover_collapses_duplicate_cursor_overlays() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    let surface = runner
        .last_paint_plan
        .primitives
        .iter_mut()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    let cursor = surface.overlays[0];
    surface.overlays.push(cursor);

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
    let surface = runner
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface) => Some(surface),
            _ => None,
        })
        .expect("gpu surface primitive");
    assert_eq!(
        surface
            .overlays
            .iter()
            .filter(|overlay| matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }))
            .count(),
        1
    );
}

#[test]
fn native_gpu_hover_preserves_app_owned_vertical_overlays() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let surface = runner
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
