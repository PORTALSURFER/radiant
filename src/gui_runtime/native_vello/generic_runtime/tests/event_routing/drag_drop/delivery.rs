use super::{fixtures::*, shared::*};

#[test]
fn captured_release_routes_pointer_drop_to_widget_under_release_point() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_release(target_point, PointerButton::Primary);

    assert_eq!(core.runtime.bridge().drops, 1);
}
