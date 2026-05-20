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
