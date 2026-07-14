use super::*;

#[test]
fn active_animation_frame_messages_are_coalesced_until_drained() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Frame ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .handle_message(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_queue_animation_frame());
    assert!(!runtime.host_queue_animation_frame());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
    assert_eq!(text_value(runtime.surface(), 10), "Frame (1)");

    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_queue_animation_frame());
    let drained = runtime.drain_runtime_messages();
    assert_eq!(drained.messages_dispatched, 1);
}

#[test]
fn animation_activity_poll_is_reused_for_frame_queue() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Polls ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .animation(|state| {
            state.count += 1;
            true
        })
        .on_frame(|| DemoMessage::Increment)
        .handle_message(|_state, _message, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    let activity = runtime.host_animation_activity();

    assert!(activity.needs_frame_message());
    assert!(runtime.host_queue_animation_frame());
    let surface = runtime.bridge_mut().project_surface();
    assert_eq!(text_value(&surface, 10), "Polls (1)");
}

#[test]
fn polling_animation_activity_does_not_queue_frame_messages() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            UiSurface::new(SurfaceNode::static_widget(TextWidget::new(
                10,
                format!("Frame ({})", state.count),
                WidgetSizing::fixed(Vector2::new(140.0, 20.0)).with_baseline(14.0),
            )))
        })
        .animation(|_| true)
        .on_frame(|| DemoMessage::Increment)
        .handle_message(|state, message, _context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert!(runtime.host_animation_activity().needs_animation());
    assert!(runtime.host_animation_activity().needs_animation());

    let drained = runtime.drain_runtime_messages();

    assert_eq!(drained.messages_dispatched, 0);
    assert_eq!(text_value(runtime.surface(), 10), "Frame (0)");
}
