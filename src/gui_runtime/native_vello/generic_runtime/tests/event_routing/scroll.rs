use super::super::*;

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
