use super::{GenericNativeVelloRunner, NativeRunOptions, Vector2, demo_bridge};
use crate::runtime::{RepaintScope, WindowEnvironmentChange};

#[test]
fn queued_window_environment_changes_defer_one_coalesced_refresh_and_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );

    runner.queue_window_environment_change(WindowEnvironmentChange::ColorSchemeOrContrast);
    runner.queue_window_environment_change(WindowEnvironmentChange::DisplayScaleOrMonitor);

    assert!(runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.timing.deferred_surface_refresh_scope,
        Some(RepaintScope::Surface)
    );
    assert!(runner.timing.deferred_scene_rebuild);
    assert!(runner.timing.deferred_scene_rebuild_requires_encode);
    assert!(!runner.timing.redraw_requested);
}

#[test]
fn dpi_change_updates_native_scale_but_defers_environment_rebuild() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );

    runner.update_native_dpi_scale(2.0);

    assert_eq!(runner.window.native_dpi_scale.factor(), 2.0);
    assert!(runner.timing.deferred_surface_refresh);
    assert!(runner.timing.deferred_scene_rebuild);
    assert!(runner.timing.deferred_scene_rebuild_requires_encode);
}

#[test]
fn accessibility_snapshot_updates_are_routed_only_for_changed_causes() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );
    let next = super::super::window_environment::AccessibilityDisplaySnapshot {
        increase_contrast: true,
        reduce_motion: false,
    };

    runner.queue_accessibility_display_snapshot(next);

    assert_eq!(
        runner.timing.deferred_surface_refresh_scope,
        Some(RepaintScope::Projection)
    );
    assert_eq!(runner.window.accessibility_display, next);
}
