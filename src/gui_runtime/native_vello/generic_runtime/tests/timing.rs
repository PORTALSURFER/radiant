use super::*;

#[test]
fn generic_core_is_repaint_driven_when_host_reports_no_animation() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    assert!(!core.animation_activity().needs_animation());
}

#[test]
fn generic_core_preserves_animation_when_host_requests_it() {
    let mut core = GenericNativeRuntimeCore::new(AnimatingBridge, Vector2::new(320.0, 40.0));

    assert!(core.animation_activity().needs_animation());
}

#[test]
fn generic_core_turns_message_free_animation_into_paint_only_redraw() {
    let mut core = GenericNativeRuntimeCore::new(AnimatingBridge, Vector2::new(320.0, 40.0));

    let activity = core.animation_activity();
    let outcome = core.drain_timed_frame(activity, false);

    assert!(!outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(!outcome.needs_scene_rebuild());
}

#[test]
fn hover_redraws_do_not_reset_timed_animation_deadline() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let now = Instant::now();
    runner.last_redraw = now;
    runner.last_timed_frame_drain = now - interval;

    let activity = runner.core.animation_activity();
    let outcome = runner.drain_timed_frame_now(now, activity, false);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert_eq!(runner.last_timed_frame_drain, now);
}

#[derive(Default)]
struct TestFrameMessageBridge {
    queued: bool,
}

impl RuntimeBridge<DemoMessage> for TestFrameMessageBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn needs_animation(&mut self) -> bool {
        true
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queued = true;
        true
    }

    fn take_runtime_messages(&mut self) -> Vec<DemoMessage> {
        if std::mem::take(&mut self.queued) {
            vec![DemoMessage::Increment]
        } else {
            Vec::new()
        }
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::request_repaint()
    }
}

#[test]
fn generic_core_turns_text_caret_animation_into_scene_rebuild_redraw() {
    let mut core = GenericNativeRuntimeCore::new(demo_bridge(), Vector2::new(320.0, 40.0));

    assert!(core.runtime.focus_widget(12));
    let outcome = core.drain_timed_frame(
        crate::runtime::RuntimeAnimationActivity::idle(),
        core.has_focused_text_input(),
    );

    assert!(!outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(outcome.needs_scene_rebuild());
}

#[test]
fn generic_runtime_clamps_animation_frame_interval() {
    assert_eq!(
        frame_cadence::animation_frame_interval(0),
        Duration::from_secs(1)
    );
    assert_eq!(
        frame_cadence::animation_frame_interval(120),
        Duration::from_secs_f64(1.0 / 120.0)
    );
    assert_eq!(
        frame_cadence::animation_frame_interval(1_000),
        Duration::from_secs_f64(1.0 / 240.0)
    );
}
