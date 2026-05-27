use super::*;
use crate::runtime::{PaintPrimitive, RuntimeAnimationActivity, TransientOverlayContext};
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
fn window_resize_events_coalesce_until_redraw_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.resize_surface(PhysicalSize::new(400, 240));
    runner.resize_surface(PhysicalSize::new(640, 360));

    assert_eq!(
        runner.timing.pending_surface_resize,
        Some(PhysicalSize::new(640, 360))
    );
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
}

#[test]
fn simple_dirty_resize_frame_can_render_directly_to_surface() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.timing.surface_resize_applied_this_frame = true;
    runner.frame.scene_texture_dirty = true;

    assert!(runner.should_render_resize_frame_directly());

    runner
        .frame
        .transient_overlay_primitives
        .push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        }));

    assert!(!runner.should_render_resize_frame_directly());
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
fn deferred_scene_rebuild_marks_pending_without_surface_refresh() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_scene_rebuild();

    assert!(runner.timing.deferred_scene_rebuild);
    assert!(!runner.timing.deferred_surface_refresh);
}

#[test]
fn deferred_viewport_resize_is_applied_at_scene_rebuild_boundary() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_viewport_resize(Vector2::new(640.0, 120.0));

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.0, 40.0));
    assert_eq!(
        runner.timing.pending_viewport_resize,
        Some(Vector2::new(640.0, 120.0))
    );

    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.viewport(), Vector2::new(640.0, 120.0));
    assert_eq!(runner.timing.pending_viewport_resize, None);
}

#[test]
fn subpixel_equivalent_resize_updates_viewport_without_relayout() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    assert!(!runner.core.set_viewport(Vector2::new(320.4, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));

    assert!(runner.core.set_viewport(Vector2::new(320.6, 40.0)));
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.6, 40.0));
}

#[test]
fn subpixel_equivalent_deferred_resize_reuses_encoded_scene() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner.frame.scene_texture_dirty = false;

    runner.defer_viewport_resize(Vector2::new(320.4, 40.0));
    runner.rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());

    assert!(!runner.timing.deferred_scene_rebuild);
    assert_eq!(runner.timing.pending_viewport_resize, None);
    assert_eq!(runner.core.runtime.viewport(), Vector2::new(320.4, 40.0));
    assert!(
        runner.frame.scene_texture_dirty,
        "the resized surface still needs a fresh texture render"
    );
}

#[test]
fn deferred_auxiliary_sync_tracks_interactive_rebuild_deferral() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        TestFrameMessageBridge::default(),
        Vector2::new(320.0, 40.0),
    );

    runner.defer_auxiliary_window_sync();

    assert!(runner.timing.deferred_auxiliary_window_sync);
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

#[test]
fn transient_overlay_hint_skips_empty_app_overlay_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
}

#[test]
fn empty_overlay_paint_skips_app_and_runtime_overlay_callbacks() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NoTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 0);
    assert!(runner.frame.transient_overlay_primitives.is_empty());
}

#[test]
fn default_transient_overlay_hint_preserves_custom_bridge_callback() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        DefaultTransientOverlayBridge::default(),
        Vector2::new(120.0, 40.0),
    );

    runner.paint_transient_overlays(&mut RenderFrameProfile::default());

    assert_eq!(runner.core.runtime.bridge().paint_calls, 1);
}

#[test]
fn frame_diagnostics_hint_can_skip_default_app_callback_work() {
    let core = GenericNativeRuntimeCore::new(NoFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(!core.has_frame_diagnostics_observer());
}

#[test]
fn default_frame_diagnostics_hint_preserves_custom_bridge_callback() {
    let core =
        GenericNativeRuntimeCore::new(DefaultFrameDiagnosticsBridge, Vector2::new(120.0, 40.0));

    assert!(core.has_frame_diagnostics_observer());
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
struct NoTransientOverlayBridge {
    paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for NoTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn has_transient_overlay_painter(&self) -> bool {
        false
    }

    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

#[derive(Default)]
struct DefaultTransientOverlayBridge {
    paint_calls: usize,
}

impl RuntimeBridge<DemoMessage> for DefaultTransientOverlayBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn paint_transient_overlay(
        &mut self,
        _context: TransientOverlayContext<'_>,
        _primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_calls += 1;
    }
}

struct NoFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for NoFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
    }

    fn has_frame_diagnostics_observer(&self) -> bool {
        false
    }
}

struct DefaultFrameDiagnosticsBridge;

impl RuntimeBridge<DemoMessage> for DefaultFrameDiagnosticsBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<DemoMessage>> {
        demo_surface(&DemoState::default())
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
