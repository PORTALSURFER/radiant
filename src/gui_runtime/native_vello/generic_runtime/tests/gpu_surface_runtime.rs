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
fn deferred_scroll_routes_message_without_refreshing_surface_until_requested() {
    let mut core =
        GenericNativeRuntimeCore::new(WheelRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let point = Point::new(12.0, 12.0);

    assert!(
        core.route_scroll_deferred_refresh(point, Vector2::new(0.0, -40.0))
            .routed
    );
    assert_eq!(core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "deferred wheel routing should not refresh the projected surface immediately"
    );

    core.refresh_surface();
    assert_eq!(core.runtime.bridge().project_count, 2);
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
fn native_gpu_hover_updates_cached_overlay_without_refreshing_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let project_count = runner.core.runtime.bridge().project_count;

    assert!(runner.update_gpu_surface_cursor_overlay(Point::new(60.0, 20.0)));
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
        GpuSurfaceOverlay::NativeHoverCursor { ratio, .. } if (*ratio - 0.25).abs() < 0.001
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
            .filter(|overlay| matches!(overlay, GpuSurfaceOverlay::NativeHoverCursor { .. }))
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
            .any(|overlay| matches!(overlay, GpuSurfaceOverlay::NativeHoverCursor { .. }))
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
    assert!(surface.capabilities.native_hover_cursor.is_some());
    assert!(
        !surface
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, GpuSurfaceOverlay::NativeHoverCursor { .. }))
    );
}

#[test]
fn queued_gpu_surface_wheel_flushes_one_coalesced_update() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let project_count = runner.core.runtime.bridge().project_count;

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, -20.0));
    runner.queue_gpu_surface_wheel(Point::new(80.0, 20.0), Vector2::new(0.0, -30.0));
    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().last_delta,
        Vector2::new(0.0, -50.0)
    );
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count,
        "coalesced wheel routing should not refresh until redraw applies deferred refresh"
    );
    assert!(runner.deferred_surface_refresh);
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

#[test]
fn signal_summary_pyramid_preserves_band_min_max_and_level_selection() {
    let samples: Arc<[f32]> = [
        -0.1, 0.2, -0.7, 0.4, 0.3, -0.8, 0.9, -0.2, -0.5, 0.1, 0.6, -0.6,
    ]
    .into_iter()
    .collect();
    let summary = GpuSignalSummary::from_interleaved_samples(&samples, 6, 2);

    assert_eq!(summary.levels[0].bucket_frames, 1);
    assert_eq!(summary.levels[0].buckets[0].min, -0.1);
    assert_eq!(summary.levels[0].buckets[0].max, -0.1);
    assert!(summary.levels.iter().any(|level| {
        level.bucket_frames >= 4 && level.buckets[0].min <= -0.7 && level.buckets[0].max >= 0.9
    }));
    assert_eq!(summary.level_for_frames_per_pixel(1.0), 0);
    assert!(summary.level_for_frames_per_pixel(5.0) > 0);
}

#[test]
fn gpu_signal_shader_uses_summary_sampling_without_looped_sample_scan() {
    assert!(!super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("loop"));
    assert!(!super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("fn band_peak("));
    assert!(super::super::gpu_surface::GPU_SIGNAL_SHADER.contains("summary_peak"));
}
