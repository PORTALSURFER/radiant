use super::*;
use crate::runtime::{RepaintScope, SurfaceInvalidation};

#[test]
fn deferred_scroll_routes_message_without_refreshing_surface_until_requested() {
    let mut core =
        GenericNativeRuntimeCore::new(WheelRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let point = Point::new(12.0, 12.0);

    assert!(
        core.route_scroll_deferred_refresh_with_modifiers(
            point,
            Vector2::new(0.0, -40.0),
            Default::default(),
        )
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
fn deferred_wheel_refresh_preserves_typed_scope_until_frame_preparation() {
    for (scope, expected_layout_passes) in
        [(RepaintScope::Projection, 0), (RepaintScope::Layout, 1)]
    {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            WheelRefreshBridge {
                repaint_scope: Some(scope),
                ..WheelRefreshBridge::default()
            },
            Vector2::new(240.0, 40.0),
        );
        let layout_before = runner.core.runtime.refresh_counters().layout;
        let outcome = runner.core.route_scroll_deferred_refresh_with_modifiers(
            Point::new(12.0, 12.0),
            Vector2::new(0.0, -40.0),
            Default::default(),
        );

        assert!(outcome.is_deferred_surface_refresh());
        assert_eq!(outcome.surface_refresh_scope_or_surface(), scope);
        runner.apply_route_outcome(outcome);
        assert_eq!(runner.timing.deferred_surface_refresh_scope, Some(scope));

        runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

        assert_eq!(
            runner.core.runtime.refresh_counters().layout,
            layout_before + expected_layout_passes
        );
        assert_eq!(
            runner.core.runtime.last_refresh_diagnostics().invalidation,
            match scope {
                RepaintScope::Projection => SurfaceInvalidation::Projection,
                RepaintScope::Layout => SurfaceInvalidation::Layout,
                RepaintScope::Surface => SurfaceInvalidation::Surface,
                RepaintScope::PaintOnly => SurfaceInvalidation::PaintOnly,
            }
        );
    }
}

#[test]
fn deferred_scroll_fallback_requests_interactive_surface_refresh() {
    let mut core =
        GenericNativeRuntimeCore::new(ScrollRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let point = Point::new(12.0, 12.0);

    let outcome = core.route_scroll_deferred_refresh_with_modifiers(
        point,
        Vector2::new(0.0, 40.0),
        Default::default(),
    );

    assert!(outcome.routed);
    assert!(!outcome.is_deferred_surface_refresh());
    assert!(outcome.is_interactive_surface_refresh());
    assert!(outcome.is_interactive_scene_rebuild());
    assert!(outcome.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "route classification should leave projection to the native runner interactive refresh path"
    );

    core.refresh_surface();
    assert_eq!(core.runtime.bridge().project_count, 2);
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

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, -20.0), Default::default());
    runner.queue_gpu_surface_wheel(
        Point::new(80.0, 20.0),
        Vector2::new(0.0, -30.0),
        Default::default(),
    );
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "queued input should not claim frame work before it is flushed"
    );
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
    assert!(runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RefreshSurface {
            reason: FrameWorkReason::DeferredSurfaceRefresh,
        },
        "flushed GPU wheel input should report the deferred refresh it schedules"
    );
}

#[test]
fn focus_loss_discards_coalesced_input_without_retaining_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, -20.0), Default::default());
    runner.queue_scroll_container_wheel(point, Vector2::new(0.0, -20.0), Default::default());
    runner.queue_scrollbar_drag(point);

    assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
    assert!(runner.input.pending_gpu_surface_wheel.is_some());
    assert!(runner.input.pending_scroll_container_wheel.is_some());
    assert!(runner.input.pending_scrollbar_drag.is_some());

    runner.handle_focus_lost_before_external_drag();

    assert!(runner.input.pending_gpu_surface_wheel.is_none());
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert!(runner.input.pending_scrollbar_drag.is_none());
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "canceled coalesced input must not leak work into presentation diagnostics"
    );
}

#[test]
fn queued_gpu_surface_wheel_refreshes_scroll_fallback_immediately() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);

    runner.queue_gpu_surface_wheel(point, Vector2::new(0.0, 40.0), Default::default());
    runner.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        2,
        "scroll fallback from a coalesced GPU region must refresh before the next present"
    );
    assert!(
        !runner.timing.deferred_surface_refresh,
        "interactive scroll fallback should not leave stale materialized rows deferred"
    );
    assert!(
        !runner.timing.deferred_scene_rebuild,
        "interactive scroll fallback should not present a stale scene"
    );
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RebuildScene {
            reason: FrameWorkReason::InteractiveSurfaceRefresh,
            mode: SceneRebuildMode::InteractiveWithSurfaceRefresh,
        },
        "coalesced wheel diagnostics should report the frame work discovered while flushing input"
    );
}
