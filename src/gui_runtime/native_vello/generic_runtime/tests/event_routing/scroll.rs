use super::super::*;
use crate::widgets::PointerModifiers;

#[test]
fn scrollbar_drag_state_survives_view_refresh_after_offset_message() {
    let mut core =
        GenericNativeRuntimeCore::new(ScrollbarBridge::default(), Vector2::new(240.0, 24.0));
    let press = Point::new(12.0, 7.0);
    let first_drag = Point::new(72.0, 7.0);
    let second_drag = Point::new(132.0, 7.0);

    assert!(
        core.route_pointer_press(press, PointerButton::Primary)
            .routed
    );
    let first_drag_outcome = core.route_pointer_move(first_drag);
    assert!(first_drag_outcome.routed);
    assert!(first_drag_outcome.needs_redraw());
    let first_offset = core.runtime.bridge().offset;
    assert!(first_offset > 0.0);

    let second_drag_outcome = core.route_pointer_move(second_drag);
    assert!(second_drag_outcome.routed);
    assert!(second_drag_outcome.needs_redraw());
    assert!(
        core.runtime.bridge().offset > first_offset,
        "drag should continue after the first offset message refreshes the surface"
    );
}

#[test]
fn scroll_area_scrollbar_drag_requests_interactive_surface_refresh() {
    let mut core =
        GenericNativeRuntimeCore::new(ScrollRefreshBridge::default(), Vector2::new(240.0, 40.0));
    let scroll_rect = core
        .runtime
        .layout()
        .rects
        .get(&61)
        .copied()
        .expect("scroll area should be laid out");
    let press = Point::new(scroll_rect.max.x - 2.0, scroll_rect.min.y + 8.0);
    let drag = Point::new(press.x, press.y + 14.0);

    core.route_pointer_press(press, PointerButton::Primary);
    let outcome = core.route_pointer_move(drag);

    assert!(!outcome.deferred_surface_refresh_requested);
    assert!(outcome.interactive_surface_refresh_requested);
    assert!(outcome.interactive_scene_rebuild_requested);
    assert!(outcome.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "scroll-area scrollbar drag should leave projection to the runner refresh path"
    );

    core.refresh_surface();
    assert_eq!(core.runtime.bridge().project_count, 2);
}

#[test]
fn scrollbar_drag_surface_refresh_rebuilds_immediately_while_captured() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ScrollRefreshBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    runner.timing.last_interactive_scene_rebuild = Instant::now();
    let scroll_rect = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&61)
        .copied()
        .expect("scroll area should be laid out");
    let press = Point::new(scroll_rect.max.x - 2.0, scroll_rect.min.y + 8.0);
    let drag = Point::new(press.x, press.y + 14.0);

    runner
        .core
        .route_pointer_press(press, PointerButton::Primary);
    let outcome = runner.core.route_pointer_move(drag);
    runner.handle_gpu_surface_pointer_move_outcome(outcome, Some(press), drag);

    assert_eq!(runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        2,
        "scrollbar drags with app-owned scroll state must refresh before the next presented scene"
    );
    assert!(
        !runner.timing.deferred_surface_refresh,
        "captured scrollbar drags should not leave a stale virtual-list surface pending"
    );
    assert!(
        !runner.timing.deferred_scene_rebuild,
        "captured scrollbar drags should not leave the scene pointing at stale materialized rows"
    );
}

#[test]
fn wheel_scroll_surface_refresh_rebuilds_immediately() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ScrollRefreshBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    runner.timing.last_interactive_scene_rebuild = Instant::now();
    let scroll_rect = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&61)
        .copied()
        .expect("scroll area should be laid out");
    let position = Point::new(scroll_rect.min.x + 20.0, scroll_rect.min.y + 20.0);

    let outcome = runner.core.route_scroll_deferred_refresh_with_modifiers(
        position,
        Vector2::new(0.0, 20.0),
        PointerModifiers::default(),
    );
    runner.handle_gpu_surface_route_outcome(outcome, position, Vector2::new(0.0, 20.0));

    assert_eq!(runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        2,
        "wheel scroll with app-owned scroll state must refresh before presenting scrolled content"
    );
    assert!(
        !runner.timing.deferred_surface_refresh,
        "wheel scroll should not leave virtual-list state pending behind runtime offset"
    );
    assert!(
        !runner.timing.deferred_scene_rebuild,
        "wheel scroll should not present stale materialized rows"
    );
}

#[test]
fn app_virtual_list_subrow_wheel_scroll_stays_runtime_local() {
    let mut core =
        GenericNativeRuntimeCore::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    let position = Point::new(20.0, 20.0);

    let outcome = core.route_scroll_deferred_refresh_with_modifiers(
        position,
        Vector2::new(0.0, 10.0),
        PointerModifiers::default(),
    );

    assert!(outcome.routed);
    assert!(outcome.needs_scene_rebuild());
    assert!(!outcome.interactive_surface_refresh_requested);
    assert_eq!(
        core.runtime.bridge().scroll_count,
        0,
        "sub-row virtual-list scrolling should not force host reprojection"
    );
    assert_eq!(core.runtime.bridge().project_count, 1);
}

#[test]
fn app_virtual_list_row_crossing_wheel_scroll_requests_surface_refresh() {
    let mut core =
        GenericNativeRuntimeCore::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    let position = Point::new(20.0, 20.0);

    let outcome = core.route_scroll_deferred_refresh_with_modifiers(
        position,
        Vector2::new(0.0, 25.0),
        PointerModifiers::default(),
    );

    assert!(outcome.routed);
    assert!(outcome.interactive_surface_refresh_requested);
    assert!(outcome.interactive_scene_rebuild_requested);
    assert_eq!(
        core.runtime.bridge().scroll_count,
        1,
        "row-crossing virtual-list scrolling must ask the host for a new materialized window"
    );
    assert_eq!(
        core.runtime.bridge().project_count,
        1,
        "surface refresh is still left to the runner so stale rows are not presented"
    );
}
