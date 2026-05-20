use radiant::{
    layout::Vector2,
    runtime::{Event, PaintPrimitive, RuntimeBridge, SurfaceRuntime},
    theme::ThemeTokens,
    widgets::PointerButton,
};

use crate::{
    model::{AnimationMessage, AnimationState},
    view::animation_view,
};

#[test]
fn animation_controls_pause_resume_and_reset_state() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 42,
        phase: 0.5,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

    click_widget(&mut runtime, 40);
    assert_status_contains(&runtime, "Paused | frame 42 | phase 0.50");

    click_widget(&mut runtime, 41);
    assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");

    click_widget(&mut runtime, 40);
    assert_status_contains(&runtime, "Running | frame 0 | phase 0.00");
}

#[test]
fn animation_controls_disable_and_reset_frame_driven_updates() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 42,
        phase: 0.5,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    assert_status_contains(&runtime, "Running | frame 43 | phase 0.24");

    click_widget(&mut runtime, 40);
    assert!(!runtime.bridge_mut().needs_animation());
    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 0);
    assert_status_contains(&runtime, "Paused | frame 43 | phase 0.24");

    click_widget(&mut runtime, 41);
    assert!(!runtime.bridge_mut().needs_animation());
    assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
}

#[test]
fn animation_control_labels_track_running_state() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 12,
        phase: 0.25,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

    assert_button_label(&runtime, 40, "Pause");
    click_widget(&mut runtime, 40);
    assert_button_label(&runtime, 40, "Run");
    click_widget(&mut runtime, 41);
    assert_button_label(&runtime, 40, "Run");
    click_widget(&mut runtime, 40);
    assert_button_label(&runtime, 40, "Pause");
}

#[test]
fn reset_stops_running_animation_on_first_frame() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 88,
        phase: 0.75,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

    click_widget(&mut runtime, 41);

    assert!(!runtime.bridge_mut().needs_animation());
    assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
    assert_button_label(&runtime, 40, "Run");
}

#[test]
fn reset_ignores_already_queued_animation_frame() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 88,
        phase: 0.75,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    click_widget(&mut runtime, 41);

    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert!(!runtime.bridge_mut().needs_animation());
    assert_status_contains(&runtime, "Paused | frame 0 | phase 0.00");
    assert_button_label(&runtime, 40, "Run");
}

#[test]
fn animation_controls_survive_pending_frame_between_press_and_release() {
    let bridge = animation_test_bridge(AnimationState {
        running: true,
        frame: 42,
        phase: 0.5,
    });
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(520.0, 220.0));
    let rect = runtime.layout().rects[&40];
    let point = rect.center();

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    let outcome = runtime.drain_runtime_messages();
    assert_eq!(outcome.messages_dispatched, 1);
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });

    assert!(!runtime.bridge_mut().needs_animation());
    assert_status_contains(&runtime, "Paused | frame 43 | phase 0.24");
}

fn animation_test_bridge(
    state: AnimationState,
) -> impl radiant::runtime::RuntimeBridge<AnimationMessage> {
    radiant::app(state)
        .view(animation_view)
        .animation(|state| state.running)
        .on_frame(|| AnimationMessage::Frame)
        .update(|state, message| match message {
            AnimationMessage::Toggle => state.running = !state.running,
            AnimationMessage::Frame => state.tick(),
            AnimationMessage::Reset => state.reset(),
        })
        .into_bridge()
}

fn click_widget<Bridge>(runtime: &mut SurfaceRuntime<Bridge, AnimationMessage>, widget_id: u64)
where
    Bridge: RuntimeBridge<AnimationMessage>,
{
    let rect = runtime.layout().rects[&widget_id];
    let point = rect.center();
    runtime.dispatch_event(Event::PointerPress {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: point,
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
}

fn assert_status_contains<Bridge>(
    runtime: &SurfaceRuntime<Bridge, AnimationMessage>,
    expected: &str,
) where
    Bridge: RuntimeBridge<AnimationMessage>,
{
    let plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.widget_id == 20 && text.text == expected
        )),
        "expected status text {expected:?}"
    );
}

fn assert_button_label<Bridge>(
    runtime: &SurfaceRuntime<Bridge, AnimationMessage>,
    widget_id: u64,
    expected: &str,
) where
    Bridge: RuntimeBridge<AnimationMessage>,
{
    let plan = runtime.paint_plan(&ThemeTokens::default());
    assert!(
        plan.primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text) if text.widget_id == widget_id && text.text == expected
        )),
        "expected button {widget_id} label {expected:?}"
    );
}
