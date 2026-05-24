use super::super::*;

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
        core.route_scroll_with_modifiers(
            canvas_point,
            Vector2::new(0.0, -40.0),
            Default::default(),
        )
        .routed
    );

    assert_eq!(core.runtime.bridge().text, "wheel");
}
