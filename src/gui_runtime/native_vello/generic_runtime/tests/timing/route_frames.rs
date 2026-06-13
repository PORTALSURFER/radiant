use super::{fixtures::*, shared::*};

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
