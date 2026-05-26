use super::*;
use crate::runtime::RuntimeAnimationActivity;
use winit::dpi::PhysicalSize;

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
    runner.timing.last_redraw = now;
    runner.timing.last_timed_frame_drain = now - interval;

    let activity = runner.core.animation_activity();
    let outcome = runner.drain_timed_frame_now(now, activity, false);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert_eq!(runner.timing.last_timed_frame_drain, now);
}

#[test]
fn pointer_routes_drain_due_frame_message_animation() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    runner.timing.last_timed_frame_drain = Instant::now() - interval;
    let mut outcome = GenericRouteOutcome {
        routed: true,
        redraw_requested: true,
        ..GenericRouteOutcome::default()
    };

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(
        outcome.repaint_requested,
        "due frame-message animation should refresh the scene even during pointer-heavy routes"
    );
}

#[test]
fn pointer_move_outcome_drain_keeps_frame_animation_moving() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let stale_deadline = Instant::now() - interval;
    runner.timing.last_timed_frame_drain = stale_deadline;

    runner.handle_gpu_surface_pointer_move_outcome(
        GenericRouteOutcome {
            routed: true,
            redraw_requested: true,
            ..GenericRouteOutcome::default()
        },
        Some(Point::new(4.0, 4.0)),
        Point::new(5.0, 4.0),
    );

    assert!(
        runner.timing.last_timed_frame_drain > stale_deadline,
        "pointer-move outcome handling should not starve due frame-message animation"
    );
}

#[test]
fn pointer_routes_do_not_overrun_timed_frame_cadence() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.timing.last_timed_frame_drain = Instant::now();
    let mut outcome = GenericRouteOutcome {
        routed: true,
        redraw_requested: true,
        ..GenericRouteOutcome::default()
    };

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(
        !outcome.repaint_requested,
        "pointer routes should not queue extra frame messages before the cadence is due"
    );
}

#[test]
fn pointer_routes_skip_animation_poll_before_native_frame_cadence_is_due() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingAnimationActivityBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(runner.options.normalized_target_fps());
    let mut outcome = GenericRouteOutcome {
        routed: true,
        redraw_requested: true,
        ..GenericRouteOutcome::default()
    };

    runner.timing.last_timed_frame_drain = Instant::now();
    runner.merge_due_timed_frame_for_route(&mut outcome);
    assert_eq!(runner.core.runtime.bridge().animation_activity_polls, 0);

    runner.timing.last_timed_frame_drain = Instant::now() - interval;
    runner.merge_due_timed_frame_for_route(&mut outcome);
    assert_eq!(runner.core.runtime.bridge().animation_activity_polls, 1);
}

#[test]
fn interactive_scene_rebuilds_are_capped_to_frame_cadence() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let now = Instant::now();
    let interval = frame_cadence::animation_frame_interval(runner.options.normalized_target_fps());

    runner.timing.last_interactive_scene_rebuild = now;
    assert!(!runner.should_rebuild_interactive_scene_now(now));

    runner.timing.last_interactive_scene_rebuild = now - interval;
    assert!(runner.should_rebuild_interactive_scene_now(now));
}

#[test]
fn surface_resizes_are_capped_to_frame_cadence() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let now = Instant::now();
    let interval = frame_cadence::animation_frame_interval(runner.options.normalized_target_fps());

    runner.timing.last_live_surface_resize = now;
    assert!(!runner.should_resize_surface_now(now));

    runner.timing.last_live_surface_resize = now - interval;
    assert!(runner.should_resize_surface_now(now));
}

#[test]
fn deferred_surface_resize_keeps_latest_nonzero_size() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_surface_resize(PhysicalSize::new(400, 240));
    runner.defer_surface_resize(PhysicalSize::new(0, 480));
    runner.defer_surface_resize(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
}

#[test]
fn deferred_interactive_scene_rebuild_is_flushed_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(runner.frame.scene_texture_dirty);
}

#[test]
fn deferred_interactive_scene_rebuild_refreshes_surface_once_before_paint() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingProjectBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let project_count = runner.core.runtime.bridge().project_count;

    runner.defer_interactive_scene_rebuild();
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
    assert_eq!(
        runner.core.runtime.bridge().project_count,
        project_count + 1,
        "deferred interactive rebuild should refresh and encode in one frame-boundary pass"
    );
}

#[derive(Default)]
struct CountingProjectBridge {
    project_count: usize,
}

impl RuntimeBridge<DemoMessage> for CountingProjectBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        self.project_count += 1;
        demo_surface(&DemoState::default())
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
}

#[derive(Default)]
struct CountingAnimationActivityBridge {
    animation_activity_polls: usize,
}

impl RuntimeBridge<DemoMessage> for CountingAnimationActivityBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.animation_activity_polls += 1;
        RuntimeAnimationActivity::idle()
    }

    fn update(&mut self, _message: DemoMessage) -> Command<DemoMessage> {
        Command::none()
    }
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
