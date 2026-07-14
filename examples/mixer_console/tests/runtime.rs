use super::*;
use radiant::runtime::Event;

#[test]
fn mixer_runtime_hover_does_not_refresh_surface() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 180.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(!second.needs_scene_rebuild());
}

#[test]
fn mixer_runtime_fader_drag_motion_uses_paint_only_preview_until_release() {
    let state = MixerState::default();
    let bridge = mixer_test_bridge(state.clone());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
    let widget = mixer_widget(&state);
    let strip = widget.strip_rect(bounds, 4);
    let fader = widget.fader_rect(strip);
    let press = Point::new(fader.center().x, fader.min.y);
    let drag = Point::new(fader.center().x, fader.min.y + fader.height() * 0.35);

    runtime.dispatch_event(Event::PointerPress {
        position: press,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    let _ = runtime.take_repaint_requested();
    let first_drag = runtime.dispatch_pointer_move_with_outcome(drag);
    assert!(first_drag.needs_scene_rebuild());
    let move_outcome =
        runtime.dispatch_pointer_move_with_outcome(Point::new(drag.x, drag.y + 12.0));

    assert!(move_outcome.paint_only_requested);
    assert!(!move_outcome.needs_scene_rebuild());

    runtime.dispatch_event(Event::PointerRelease {
        position: drag,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    assert!(runtime.take_repaint_requested());
}

#[test]
fn mixer_runtime_frame_messages_advance_status() {
    let bridge = mixer_test_bridge(MixerState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}
