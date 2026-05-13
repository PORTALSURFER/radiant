use super::*;

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
