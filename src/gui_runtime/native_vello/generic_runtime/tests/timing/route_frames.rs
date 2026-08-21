use super::super::super::frame_scheduler_policy::NativeInputStageDisposition;
use super::super::super::runner_state::NativeSurfaceAcquireFailure;
use super::{fixtures::*, shared::*};

fn over_budget(outcome: GenericRouteOutcome) -> GenericRouteOutcome {
    outcome.with_native_input_stage_disposition(NativeInputStageDisposition::DeferLowerPriority)
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
        ..GenericRouteOutcome::default()
    };
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert!(
        outcome.needs_scene_rebuild(),
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
fn exceeded_input_defers_all_visual_route_work_but_not_exit_or_noop() {
    let mut cases = Vec::new();

    cases.push(GenericRouteOutcome::default());

    let mut paint_only = GenericRouteOutcome::default();
    paint_only.request_paint_only(FrameWorkReason::RuntimePaintOnly);
    cases.push(paint_only);

    cases.push(GenericRouteOutcome {
        frame_work: FrameWork::ResizeSurface {
            reason: FrameWorkReason::NativeResize,
        },
        ..GenericRouteOutcome::default()
    });

    let mut refresh = GenericRouteOutcome::default();
    refresh.request_surface_refresh(FrameWorkReason::DeferredSurfaceRefresh);
    cases.push(refresh);

    let mut resize_and_rebuild = GenericRouteOutcome::default();
    resize_and_rebuild.request_resize_and_rebuild(FrameWorkReason::NativeResize);
    cases.push(resize_and_rebuild);

    for mode in [
        SceneRebuildMode::Immediate,
        SceneRebuildMode::ImmediateWithSurfaceRefresh,
        SceneRebuildMode::Interactive,
        SceneRebuildMode::InteractiveWithSurfaceRefresh,
    ] {
        cases.push(GenericRouteOutcome {
            frame_work: FrameWork::RebuildScene {
                reason: FrameWorkReason::RoutedInput,
                mode,
            },
            ..GenericRouteOutcome::default()
        });
    }

    let mut exit = GenericRouteOutcome::default();
    exit.request_exit();
    cases.push(exit);

    for outcome in cases {
        let mut runner = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            TestFrameMessageBridge::default(),
            Vector2::new(320.0, 40.0),
        );
        let counters_before = runner.core.runtime.refresh_counters();
        let frame_work = outcome.frame_work();
        let applied = runner.apply_route_outcome_with_timed_frame(over_budget(outcome), false);

        assert_eq!(runner.core.runtime.refresh_counters(), counters_before);
        if matches!(frame_work, FrameWork::Exit { .. }) {
            assert!(applied.exit_requested);
            assert!(!runner.timing.deferred_surface_refresh);
            assert!(!runner.timing.deferred_scene_rebuild);
            assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
        } else if matches!(frame_work, FrameWork::None) {
            assert!(!applied.exit_requested);
            assert!(!runner.timing.deferred_surface_refresh);
            assert!(!runner.timing.deferred_scene_rebuild);
            assert_eq!(runner.timing.pending_frame_work, FrameWork::None);
            assert!(!runner.timing.redraw_requested);
        } else {
            assert!(!applied.exit_requested);
            assert_eq!(runner.timing.pending_frame_work, frame_work);
            if matches!(
                frame_work,
                FrameWork::RefreshSurface { .. }
                    | FrameWork::RebuildScene {
                        mode: SceneRebuildMode::ImmediateWithSurfaceRefresh
                            | SceneRebuildMode::InteractiveWithSurfaceRefresh,
                        ..
                    }
            ) {
                assert!(runner.timing.deferred_surface_refresh);
            }
            if matches!(
                frame_work,
                FrameWork::ResizeAndRebuild { .. } | FrameWork::RebuildScene { .. }
            ) {
                assert!(runner.timing.deferred_scene_rebuild);
                assert!(runner.timing.deferred_auxiliary_window_sync);
            }
        }
    }
}

#[test]
fn exceeded_input_leaves_due_deadline_for_the_later_native_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let last_drain = Instant::now() - interval;
    runner.timing.last_timed_frame_drain = last_drain;

    runner.apply_route_outcome_with_timed_frame(over_budget(GenericRouteOutcome::default()), true);

    assert_eq!(runner.timing.last_timed_frame_drain, last_drain);

    runner.apply_route_outcome_with_timed_frame(GenericRouteOutcome::default(), true);

    assert!(runner.timing.last_timed_frame_drain > last_drain);
}

#[test]
fn two_exceeded_inputs_keep_both_completions_in_one_bounded_visual_state() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let counters_before = runner.core.runtime.refresh_counters();

    for reason in [
        FrameWorkReason::RoutedInput,
        FrameWorkReason::RuntimeSurfaceRepaint,
    ] {
        let mut outcome = GenericRouteOutcome::default();
        outcome.request_scene_rebuild(reason);
        runner.apply_route_outcome_with_timed_frame(over_budget(outcome), false);
    }

    assert!(runner.timing.deferred_scene_rebuild);
    assert!(runner.timing.deferred_auxiliary_window_sync);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        }
    );
    assert_eq!(runner.core.runtime.refresh_counters(), counters_before);
}

#[test]
fn deferred_exceeded_rebuild_is_consumed_once_at_the_later_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let mut outcome = GenericRouteOutcome::default();
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);
    runner.apply_route_outcome_with_timed_frame(over_budget(outcome), false);
    assert!(runner.timing.deferred_scene_rebuild);

    let mut first_profile = RenderFrameProfile::default();
    assert!(runner.rebuild_deferred_scene_if_needed(&mut first_profile));
    assert!(!runner.timing.deferred_scene_rebuild);
    let counters_after_first = runner.core.runtime.refresh_counters();

    let mut second_profile = RenderFrameProfile::default();
    assert!(!runner.rebuild_deferred_scene_if_needed(&mut second_profile));
    assert_eq!(runner.core.runtime.refresh_counters(), counters_after_first);
}

#[test]
fn exceeded_visual_primary_waits_for_presentation_before_native_requests() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let mut outcome = GenericRouteOutcome::default();
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);
    outcome.runtime_work_remaining = true;
    let frame_work = outcome.frame_work();

    runner.apply_route_outcome_with_timed_frame(over_budget(outcome), false);

    assert!(!runner.timing.redraw_requested);
    assert!(!runner.runtime_wakeup.is_pending());
    assert_eq!(runner.timing.pending_frame_work, frame_work);
    assert!(runner.timing.deferred_scene_rebuild);
    assert!(runner.timing.deferred_auxiliary_window_sync);

    let mut profile = RenderFrameProfile::default();
    assert!(runner.rebuild_deferred_scene_if_needed(&mut profile));
    assert_eq!(runner.take_pending_frame_work(), frame_work);
    assert_eq!(runner.take_pending_frame_work(), FrameWork::None);
    assert!(!runner.timing.deferred_scene_rebuild);
}

#[test]
fn native_resize_redraw_waits_for_confirmed_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.resize_surface(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "native resize redraws should not report resize work before a surface size change is applied"
    );
}

#[test]
fn frame_diagnostics_redraw_requests_skip_tracking_without_observer() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoFrameDiagnosticsBridge,
        Vector2::new(320.0, 40.0),
    );

    runner.request_redraw_for_frame_work(FrameWork::PaintOnly {
        reason: FrameWorkReason::PointerHover,
    });
    runner.request_redraw_for_frame_work(FrameWork::ResizeAndRebuild {
        reason: FrameWorkReason::NativeResize,
    });

    let profile = RenderFrameProfile::recording(runner.frame_diagnostics_enabled);

    assert!(!runner.frame_diagnostics_enabled);
    assert!(!profile.record_timings);
    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::None,
        "redraw requests must not mutate diagnostics-only state when no observer is registered"
    );
}

#[test]
fn unbound_native_resources_suppress_redraw_reissue_but_retain_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let work = FrameWork::PaintOnly {
        reason: FrameWorkReason::PointerHover,
    };

    runner.request_redraw_for_frame_work(work);

    assert_eq!(runner.timing.pending_frame_work, work);
    assert!(!runner.timing.redraw_requested);
    assert!(runner.window.native_resources.is_none());
}

#[test]
fn frame_diagnostics_availability_is_cached_at_runner_start() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        CountingFrameDiagnosticsBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.request_redraw_for_frame_work(FrameWork::PaintOnly {
        reason: FrameWorkReason::PointerHover,
    });
    assert_eq!(runner.take_pending_frame_work(), FrameWork::None);

    assert_eq!(
        runner.core.runtime.bridge().observer_checks.get(),
        1,
        "hot redraw and presentation paths should reuse the startup capability check"
    );
}

#[test]
fn pending_redraw_frame_work_merges_stronger_direct_request() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    runner.timing.pending_frame_work = FrameWork::PaintOnly {
        reason: FrameWorkReason::PointerHover,
    };

    runner.request_redraw_for_frame_work(FrameWork::ResizeAndRebuild {
        reason: FrameWorkReason::NativeResize,
    });

    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::ResizeAndRebuild {
            reason: FrameWorkReason::NativeResize
        },
        "later direct resize work should not be hidden by an earlier paint-only redraw"
    );
}

#[test]
fn coalesced_timeout_retry_keeps_pending_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let pending = FrameWork::ResizeAndRebuild {
        reason: FrameWorkReason::NativeResize,
    };
    runner.timing.pending_frame_work = pending;

    // Timeout recovery requests a redraw with no new frame-work reason. The
    // existing coalescing path must retain work from the failed acquisition.
    runner.request_redraw_for_frame_work(FrameWork::None);

    assert_eq!(runner.timing.pending_frame_work, pending);
}

#[test]
fn timeout_and_other_retries_share_one_transient_permit() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner
        .window
        .surface_recovery
        .observe_acquire_error(&NativeSurfaceAcquireFailure::Timeout);
    assert!(
        runner
            .window
            .surface_recovery
            .record_timeout_retry_request(true)
    );
    runner
        .window
        .surface_recovery
        .observe_acquire_error(&NativeSurfaceAcquireFailure::Other);
    assert!(
        !runner
            .window
            .surface_recovery
            .record_other_retry_request(true)
    );

    runner.window.surface_recovery.rearm_transient_retry();
    runner
        .window
        .surface_recovery
        .observe_acquire_error(&NativeSurfaceAcquireFailure::Other);
    assert!(
        runner
            .window
            .surface_recovery
            .record_other_retry_request(true)
    );
    runner
        .window
        .surface_recovery
        .observe_acquire_error(&NativeSurfaceAcquireFailure::Timeout);
    assert!(
        !runner
            .window
            .surface_recovery
            .record_timeout_retry_request(true)
    );

    let diagnostics = runner.window.surface_recovery.diagnostics();
    assert_eq!(diagnostics.timeouts, 2);
    assert_eq!(diagnostics.others, 2);
    assert_eq!(diagnostics.timeout_retry_requests, 1);
    assert_eq!(diagnostics.other_retry_requests, 1);
}

#[test]
fn coalesced_routed_redraws_keep_strongest_frame_work() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let mut rebuild = GenericRouteOutcome::default();
    rebuild.request_scene_rebuild(FrameWorkReason::RuntimeSurfaceRepaint);
    let mut paint_only = GenericRouteOutcome::default();
    paint_only.request_paint_only(FrameWorkReason::RuntimePaintOnly);

    runner.apply_route_outcome(rebuild);
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    runner.apply_route_outcome(paint_only);

    assert_eq!(
        runner.timing.pending_frame_work,
        FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        },
        "paint-only routes coalesced behind a pending redraw must not hide scene work"
    );
}

#[test]
fn auxiliary_message_route_does_not_admit_a_second_due_timed_frame() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let interval = frame_cadence::animation_frame_interval(60);
    let last_drain = Instant::now() - interval;
    runner.timing.last_timed_frame_drain = last_drain;
    let outcome = GenericRouteOutcome {
        routed: true,
        ..GenericRouteOutcome::default()
    };

    runner.apply_route_outcome_with_timed_frame(outcome, false);

    assert_eq!(
        runner.timing.last_timed_frame_drain, last_drain,
        "parent dispatch of an auxiliary timed message must not consume a second timed admission"
    );
}

#[test]
fn primary_and_auxiliary_redraw_observations_finalize_once_per_stable_key() {
    let mut ledger = CpuFrameObservationLedger::default();
    for key in [
        FrameScheduleKey::Primary,
        FrameScheduleKey::Auxiliary(String::from("settings")),
    ] {
        let admission = ledger.begin(
            key.clone(),
            FrameWork::PaintOnly {
                reason: FrameWorkReason::PointerHover,
            },
            Some(60),
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_stage(
            CpuFrameStage::SubmitPresent,
            true,
            CpuFrameDuration::Unknown,
        );
        capture.mark_successful_presentation();
        ledger.finish(admission, capture, false);

        let counters = ledger
            .counters_for_test(&key)
            .expect("route admission should retain stable-key evidence");
        assert_eq!(counters.admitted_redraws, 1);
        assert_eq!(counters.successful_presentations, 1);
        assert_eq!(counters.failed_frames, 0);
    }
}

#[test]
fn auxiliary_route_time_stale_redraw_commits_once_to_parent_observation() {
    for (redraw_failed, recovery_triggered) in [(false, false), (true, true)] {
        let mut parent_ledger = CpuFrameObservationLedger::default();
        let key = FrameScheduleKey::Auxiliary(String::from("settings"));
        let mut child = GenericNativeVelloRunner::new_auxiliary(
            NativeRunOptions::default(),
            TestFrameMessageBridge::default(),
            Vector2::new(320.0, 40.0),
            String::from("settings"),
        );

        let now = Instant::now();
        child.timing.redraw_requested = true;
        child.timing.redraw_requested_at = Some(now - Duration::from_millis(17));
        let pending_at_route_start = child
            .pending_redraw_elapsed(now)
            .expect("route should observe the pending auxiliary redraw");
        let mut route_outcome = GenericRouteOutcome {
            routed: true,
            ..GenericRouteOutcome::default()
        };
        route_outcome.request_paint_only(FrameWorkReason::RoutedInput);
        child.apply_route_outcome(route_outcome);

        assert!(
            child.should_flush_pending_redraw_after_route(
                pending_at_route_start,
                Duration::from_millis(1)
            ),
            "a stale non-RedrawRequested route should take the synchronous flush branch"
        );
        assert!(
            child.cpu_frame_observation.is_none(),
            "the auxiliary child must not become a second ledger owner"
        );

        let mut observation = CpuFrameObservationOwner::new(&mut parent_ledger, key.clone());
        let admission = child.begin_cpu_frame_observation_with_owner(&mut observation, now);
        child
            .cpu_frame_observation_capture
            .mark_frame_path_started();
        child.cpu_frame_observation_capture.record_stage(
            CpuFrameStage::SubmitPresent,
            true,
            CpuFrameDuration::Unknown,
        );
        if recovery_triggered {
            child.mark_cpu_frame_observation_recovery();
        } else {
            child
                .cpu_frame_observation_capture
                .mark_successful_presentation();
        }
        child.finish_cpu_frame_observation_with_owner(&mut observation, admission, redraw_failed);
        drop(observation);

        let counters = parent_ledger
            .counters_for_test(&key)
            .expect("parent-owned route flush should retain auxiliary evidence");
        assert_eq!(
            parent_ledger.sample_count_for_test(&key),
            Some(1),
            "one synchronous auxiliary route flush should append one parent sample"
        );
        assert_eq!(counters.admitted_redraws, 1);
        assert_eq!(
            counters.successful_presentations,
            u64::from(!redraw_failed),
            "successful route flushes should complete exactly once"
        );
        assert_eq!(counters.failed_frames, u64::from(redraw_failed));
        assert_eq!(
            counters.recovery_triggered_frames,
            u64::from(recovery_triggered)
        );
    }
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
        {
            let mut outcome = GenericRouteOutcome {
                routed: true,
                ..GenericRouteOutcome::default()
            };
            outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);
            outcome
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
        ..GenericRouteOutcome::default()
    };
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);

    runner.merge_due_timed_frame_for_route(&mut outcome);

    assert!(outcome.routed);
    assert!(outcome.needs_redraw());
    assert_eq!(
        outcome.frame_work_reason(),
        "routed_input",
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
        ..GenericRouteOutcome::default()
    };
    outcome.request_scene_rebuild(FrameWorkReason::RoutedInput);

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
fn frame_wait_deadline_suppresses_retry_without_current_adapter_bundle() {
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
        scheduled,
        "a missing primary adapter/resource generation must remain quiescent"
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

#[test]
fn exceeded_route_does_not_reissue_a_stale_pending_redraw() {
    let runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let mut outcome = GenericRouteOutcome::default();
    outcome.request_paint_only(FrameWorkReason::RoutedInput);

    assert!(!runner.should_flush_pending_redraw_for_route_outcome(
        over_budget(outcome),
        Duration::from_millis(17),
        Duration::from_millis(1),
    ));
    assert!(runner.should_flush_pending_redraw_for_route_outcome(
        outcome,
        Duration::from_millis(17),
        Duration::from_millis(1),
    ));
}
