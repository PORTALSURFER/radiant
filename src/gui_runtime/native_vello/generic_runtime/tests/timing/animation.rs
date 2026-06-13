use super::shared::*;

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
