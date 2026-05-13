use super::*;

#[test]
fn generic_core_routes_pointer_and_key_input_to_host_messages() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );
    assert_eq!(core.runtime.bridge().state.count, 1);

    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_character('R').routed);
    assert!(core.route_character(' ').routed);
    assert!(core.route_widget_key(WidgetKey::Enter).routed);
    assert_eq!(core.runtime.bridge().state.name, "R ");
}

#[test]
fn nested_button_activation_survives_surface_refresh_between_press_and_release() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    core.runtime.refresh();
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().state.count, 1);
}

#[test]
fn generic_core_routes_text_edit_commands_only_to_text_inputs() {
    let bridge = demo_bridge();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(!core.route_text_edit(TextEditCommand::SelectAll).routed);

    let input_point = core
        .runtime
        .layout()
        .rects
        .get(&12)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("text input should be laid out");
    assert!(
        core.route_pointer_press(input_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_text_edit(TextEditCommand::SelectAll).routed);
}

#[test]
fn generic_canvas_can_receive_keyboard_focus_and_text_input() {
    let bridge = CanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&21)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(core.runtime.surface().keyboard_focus_order().contains(&21));
    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(core.route_character('K').routed);

    assert_eq!(core.runtime.bridge().text, "K");
}

#[test]
fn generic_canvas_receives_wheel_before_scroll_fallback() {
    let bridge = CanvasBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let canvas_point = core
        .runtime
        .layout()
        .rects
        .get(&21)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("canvas should be laid out");

    assert!(
        core.route_scroll(canvas_point, Vector2::new(0.0, -40.0))
            .routed
    );

    assert_eq!(core.runtime.bridge().text, "wheel");
}

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
fn generic_core_drains_command_repaint_requests_after_routing() {
    let bridge = RepaintBridge::default();
    let mut core = GenericNativeRuntimeCore::new(bridge, Vector2::new(320.0, 40.0));
    let button_point = core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 2.0, rect.min.y + 2.0))
        .expect("button should be laid out");

    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_release(button_point, PointerButton::Primary);

    assert!(outcome.routed);
    assert!(outcome.repaint_requested);
    assert!(!core.runtime.repaint_requested());
    assert_eq!(core.runtime.bridge().state.count, 1);
}
