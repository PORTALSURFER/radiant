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
fn due_frame_animation_waits_behind_fresh_pending_redraw() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let last_drain = Instant::now() - interval;
    runner.timing.last_timed_frame_drain = last_drain;
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    let mut outcome = GenericRouteOutcome::default();

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert_eq!(
        runner.timing.last_timed_frame_drain, last_drain,
        "pending presentation should keep timed animation from consuming a hidden frame"
    );
    assert_eq!(
        outcome,
        GenericRouteOutcome::default(),
        "fresh pending redraws already have a visible frame in flight"
    );
}

#[test]
fn stale_pending_redraw_does_not_block_due_frame_animation() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let last_drain = Instant::now() - interval;
    runner.timing.last_timed_frame_drain = last_drain;
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(17));
    let mut outcome = GenericRouteOutcome::default();

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert!(
        runner.timing.last_timed_frame_drain > last_drain,
        "stale pending redraws should keep the recovery path moving"
    );
    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
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
fn pending_redraw_requests_are_reissued_when_input_starves_present() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let now = Instant::now();

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(now);
    assert!(
        !runner.pending_redraw_request_is_stale(now + Duration::from_millis(8)),
        "fresh pending redraws should still coalesce"
    );
    assert!(
        runner.pending_redraw_request_is_stale(now + Duration::from_millis(17)),
        "stale pending redraws should be reissued during sustained input bursts"
    );
}

#[test]
fn pending_redraw_elapsed_tracks_present_age() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let now = Instant::now();

    assert_eq!(runner.pending_redraw_elapsed(now), None);

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(now);
    assert_eq!(
        runner.pending_redraw_elapsed(now + Duration::from_millis(8)),
        Some(Duration::from_millis(8)),
        "fresh pending redraw age should be available to route-time flushes"
    );
    assert_eq!(
        runner.pending_redraw_elapsed(now + Duration::from_millis(17)),
        Some(Duration::from_millis(17)),
        "stale pending redraw age should still be available to route-time flushes"
    );
}

#[test]
fn frame_wait_deadline_includes_pending_redraw_reissue_deadline() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let now = Instant::now();
    let scheduled = now + Duration::from_millis(30);

    assert_eq!(runner.frame_wait_deadline(scheduled), scheduled);

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(now);

    assert_eq!(
        runner.frame_wait_deadline(scheduled),
        now + Duration::from_millis(16),
        "animation waits should wake early enough to recover a swallowed redraw"
    );
}

#[test]
fn route_time_redraw_flush_waits_for_stale_request() {
    let runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let frame_interval =
        frame_cadence::animation_frame_interval(runner.options.normalized_target_fps());

    assert!(
        !runner
            .should_flush_pending_redraw_after_route(Duration::from_millis(1), frame_interval / 2),
        "fresh redraws should not force an extra present inside the current frame slot"
    );
    assert!(
        !runner.should_flush_pending_redraw_after_route(Duration::from_millis(1), frame_interval),
        "fresh redraws should stay on the native redraw path even when the last present is old"
    );
    assert!(
        runner.should_flush_pending_redraw_after_route(
            Duration::from_millis(17),
            Duration::from_millis(1)
        ),
        "stale redraw requests should be flushed even when the last present was recent"
    );
}
