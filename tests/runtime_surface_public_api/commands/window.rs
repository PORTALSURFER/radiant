use super::fixtures::RuntimeCommandBridge;
use super::*;

#[test]
fn surface_runtime_reports_dpi_scale_overrides_as_surface_refreshes() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let outcome = runtime.execute_command(Command::set_dpi_scale(DpiScale::new(2.0)));

    assert_eq!(outcome.dpi_scale_override, Some(DpiScale::new(2.0)));
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(!outcome.paint_only_requested);
}

#[test]
fn surface_runtime_reports_window_size_requests_as_surface_refreshes() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));

    let requested = Vector2::new(760.0, 520.0);
    let outcome = runtime.execute_command(Command::set_window_logical_size(requested));

    assert_eq!(outcome.window_logical_size, Some(requested));
    assert!(outcome.repaint_requested);
    assert!(outcome.surface_repaint_requested);
    assert!(outcome.surface_refresh_requested);
    assert!(!outcome.paint_only_requested);
}

#[test]
fn typed_refreshes_do_not_narrow_window_or_dpi_surface_fallbacks() {
    let bridge = RuntimeCommandBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 80.0));
    let initial_layout_count = runtime.refresh_counters().layout;

    let dpi = runtime.execute_command(Command::batch([
        Command::set_dpi_scale(DpiScale::new(2.0)),
        Command::repaint(RepaintScope::Projection),
    ]));

    assert_eq!(
        dpi.surface_invalidation(),
        radiant::runtime::SurfaceInvalidation::Surface
    );
    assert_eq!(runtime.refresh_counters().layout, initial_layout_count + 1);

    let window = runtime.execute_command(Command::batch([
        Command::set_window_logical_size(Vector2::new(760.0, 520.0)),
        Command::repaint(RepaintScope::Layout),
    ]));

    assert_eq!(
        window.surface_invalidation(),
        radiant::runtime::SurfaceInvalidation::Surface
    );
    assert_eq!(runtime.refresh_counters().layout, initial_layout_count + 2);
}
