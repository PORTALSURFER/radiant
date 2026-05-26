use super::{
    super::{GenericNativeRuntimeCore, GenericNativeVelloRunner, RenderFrameProfile, demo_bridge},
    fixtures::{LocalPointerMoveBridge, PointerMoveBridge},
};
use crate::{
    layout::{Point, Vector2},
    runtime::NativeRunOptions,
    widgets::PointerButton,
};

#[test]
fn pointer_move_inside_same_widget_does_not_request_redundant_redraw() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_rect = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .copied()
        .expect("button should be laid out");
    let first_point = Point::new(button_rect.min.x + 2.0, button_rect.min.y + 2.0);
    let second_point = Point::new(button_rect.min.x + 4.0, button_rect.min.y + 2.0);

    let first = core.route_pointer_move(first_point);
    assert!(first.routed);
    assert!(first.needs_redraw());

    let second = core.route_pointer_move(second_point);
    assert!(second.routed);
    assert!(!second.needs_redraw());
}

#[test]
fn pointer_move_message_inside_same_widget_still_requests_redraw() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
    assert_eq!(core.runtime.bridge().moves, 2);
}

#[test]
fn pointer_move_messages_defer_surface_refresh_until_redraw_after_hover_enters() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    assert_eq!(core.runtime.bridge().project_count, 1);
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.needs_scene_rebuild());
    assert_eq!(core.runtime.bridge().project_count, 2);

    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
    assert!(!second.needs_scene_rebuild());
    assert!(second.deferred_surface_refresh_requested);
    assert_eq!(core.runtime.bridge().moves, 2);
    assert_eq!(
        core.runtime.bridge().project_count,
        2,
        "stable pointer-move messages should reduce immediately but coalesce surface projection until redraw"
    );
}

#[test]
fn captured_pointer_move_messages_defer_surface_refresh_until_redraw_for_resizes() {
    let mut core =
        GenericNativeRuntimeCore::new(PointerMoveBridge::default(), Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let press = core.route_pointer_press(point, PointerButton::Primary);
    assert!(press.routed);

    let project_count_before_move = core.runtime.bridge().project_count;
    let drag_move = core.route_pointer_move(Point::new(point.x + 4.0, point.y));

    assert!(drag_move.routed);
    assert!(drag_move.needs_redraw());
    assert!(!drag_move.needs_scene_rebuild());
    assert!(drag_move.deferred_surface_refresh_requested);
    assert_eq!(core.runtime.bridge().moves, 1);
    assert_eq!(
        core.runtime.bridge().project_count,
        project_count_before_move,
        "captured pointer messages should reduce immediately but coalesce surface projection until redraw"
    );
}

#[test]
fn deferred_pointer_move_refresh_invalidates_scene_texture() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        PointerMoveBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;
    let point = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&71)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("pointer widget should be laid out");

    let first = runner.core.route_pointer_move(point);
    assert!(first.needs_scene_rebuild());
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;
    runner.frame.composited_base_dirty = false;

    let second = runner
        .core
        .route_pointer_move(Point::new(point.x + 1.0, point.y));
    assert!(second.deferred_surface_refresh_requested);
    runner.timing.deferred_surface_refresh = true;
    runner.refresh_deferred_surface_if_needed(&mut RenderFrameProfile::default());

    assert!(runner.frame.scene_texture_dirty);
    assert!(runner.frame.composited_base_dirty);
}

#[test]
fn local_pointer_move_state_inside_same_widget_requests_redraw() {
    let mut core = GenericNativeRuntimeCore::new(LocalPointerMoveBridge, Vector2::new(120.0, 40.0));
    let point = core
        .runtime
        .layout()
        .rects
        .get(&72)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("local pointer widget should be laid out");

    let first = core.route_pointer_move(point);
    assert!(first.routed);
    assert!(first.needs_redraw());
    let second = core.route_pointer_move(Point::new(point.x + 1.0, point.y));

    assert!(second.routed);
    assert!(second.needs_redraw());
}
