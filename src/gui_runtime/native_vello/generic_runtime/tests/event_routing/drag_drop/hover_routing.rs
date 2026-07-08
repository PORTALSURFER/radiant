use super::{fixtures::*, shared::*};

#[test]
fn captured_drag_routes_pointer_move_to_hovered_drop_target() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 1);
    assert_eq!(core.runtime.pointer_capture(), Some(71));
}

#[test]
fn captured_drag_routes_pointer_move_to_drop_target_after_surface_refresh() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    core.runtime.refresh();
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 1);
    assert_eq!(core.runtime.pointer_capture(), Some(71));
}

#[test]
fn captured_drag_handle_does_not_route_pointer_move_to_hovered_widget() {
    let mut core = GenericNativeRuntimeCore::new(
        DragHandlePassThroughBridge::default(),
        Vector2::new(220.0, 32.0),
    );
    let source_point = widget_point(&core, 81, "drag handle");
    let target_point = widget_point(&core, 82, "hover target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let outcome = core.route_pointer_move(target_point);

    assert!(outcome.routed);
    assert_eq!(core.runtime.bridge().hovers, 0);
    assert_eq!(core.runtime.pointer_capture(), Some(81));
}

#[test]
fn captured_drag_hover_message_requests_scene_rebuild_without_hover_change() {
    let mut core = GenericNativeRuntimeCore::new(DropBridge::default(), Vector2::new(220.0, 32.0));
    let source_point = widget_point(&core, 71, "source");
    let target_point = widget_point(&core, 72, "target");

    assert!(
        core.route_pointer_press(source_point, PointerButton::Primary)
            .routed
    );
    let _ = core.route_pointer_move(target_point);
    let outcome = core.route_pointer_move(Point::new(target_point.x + 2.0, target_point.y));

    assert!(outcome.routed);
    assert!(
        outcome.needs_scene_rebuild(),
        "captured drag hover messages mutate app state and must rebuild the scene, not only repaint the drag preview"
    );
    assert!(!outcome.is_paint_only());
    assert_eq!(core.runtime.bridge().hovers, 2);
}
