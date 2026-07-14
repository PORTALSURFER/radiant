use super::*;

#[test]
fn piano_roll_runtime_routes_mouse_wheel_to_viewport_zoom() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let initial_status = status_text(&runtime);

    assert!(runtime.wheel_or_scroll_at(bounds.center(), Vector2::new(0.0, -40.0)));

    let next_status = status_text(&runtime);
    assert_ne!(next_status, initial_status);
    assert!(
        next_status.contains("beats 0.0-16.0") && next_status.contains("pitches C#3-A#4"),
        "live wheel routing should reach the piano roll widget and zoom pitch only; got {next_status}"
    );
}

#[test]
fn piano_roll_runtime_routes_alt_mouse_wheel_to_time_zoom() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let initial_status = status_text(&runtime);

    assert!(runtime.wheel_or_scroll_at_with_modifiers(
        bounds.center(),
        Vector2::new(0.0, -40.0),
        PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        }
    ));

    let next_status = status_text(&runtime);
    assert_ne!(next_status, initial_status);
    assert!(
        next_status.contains("beats 1.6-14.4") && next_status.contains("pitches C3-B4"),
        "alt wheel routing should reach the piano roll widget and zoom time only; got {next_status}"
    );
}

#[test]
fn piano_roll_runtime_hover_does_not_refresh_surface() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 160.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 260.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable piano-roll hover should avoid reprojection and full scene rebuilds"
    );
}

#[test]
fn piano_roll_runtime_frame_messages_advance_status() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}
