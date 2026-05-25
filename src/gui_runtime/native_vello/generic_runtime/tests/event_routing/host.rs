use super::super::*;
use crate::widgets::TextEditCommand;

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
fn generic_core_routes_second_nearby_press_as_double_click() {
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
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(canvas_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_press(canvas_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().text, "double");
}

#[test]
fn second_press_falls_back_to_normal_press_when_widget_ignores_double_click() {
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
    assert!(
        core.route_pointer_press(button_point, PointerButton::Primary)
            .routed
    );
    assert!(
        core.route_pointer_release(button_point, PointerButton::Primary)
            .routed
    );

    assert_eq!(core.runtime.bridge().state.count, 2);
}
