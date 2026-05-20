use super::super::*;

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
