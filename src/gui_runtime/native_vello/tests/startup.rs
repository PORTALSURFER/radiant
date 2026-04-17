use super::*;

#[test]
fn deferred_startup_fallback_defers_model_and_overlay_pulls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;
    runner.frame_state.mark_layout_dirty();
    runner.frame_state.mark_model_dirty();

    runner.prepare_startup_first_frame_scene();

    assert!(!runner.frame_state.take_model());
    assert!(runner.frame_state.take_scene());
    assert!(!runner.frame_state.take_state_overlay());
    assert!(!runner.frame_state.take_motion_overlay());
}

#[test]
fn startup_placeholder_scene_uses_theme_clear_color_and_branding() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);

    runner.build_startup_placeholder_scene(&layout, &style);

    assert_eq!(runner.clear_color, style.clear_color);
    assert_eq!(runner.frame_cache.clear_color, style.clear_color);
    assert_eq!(runner.frame_cache.text_runs.len(), 2);
    assert_eq!(runner.hover_overlay_frame_cache.text_runs.len(), 0);
    assert_eq!(runner.focus_overlay_frame_cache.text_runs.len(), 0);
    assert_eq!(runner.modal_overlay_frame_cache.text_runs.len(), 0);
    assert_eq!(
        runner.waveform_motion_overlay_frame_cache.text_runs.len(),
        0
    );
    assert_eq!(runner.chrome_motion_overlay_frame_cache.text_runs.len(), 0);
    assert!(
        runner
            .frame_cache
            .text_runs
            .iter()
            .any(|run| run.text == crate::app::DEFAULT_APP_TITLE)
    );
}

#[test]
fn deferred_startup_fallback_rebuild_uses_placeholder_scene_before_first_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);
    runner.shell_layout = Some(Arc::new(layout.clone()));
    runner.style_cache = Some(style);
    runner.frame_state.scene_dirty = true;
    runner.frame_state.model_dirty = false;
    runner.frame_state.state_overlay_dirty = false;
    runner.frame_state.motion_overlay_dirty = false;
    runner.startup_model_pull_pending = true;
    runner.first_frame_presented = false;

    runner.rebuild_scene_if_needed();

    assert!(
        runner
            .frame_cache
            .text_runs
            .iter()
            .any(|run| run.text.contains("Starting interface"))
    );
}

#[test]
fn startup_default_rebuild_skips_placeholder_scene_before_first_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);
    runner.shell_layout = Some(Arc::new(layout));
    runner.style_cache = Some(style);
    runner.frame_state.scene_dirty = true;
    runner.frame_state.model_dirty = false;
    runner.frame_state.state_overlay_dirty = false;
    runner.frame_state.motion_overlay_dirty = false;
    runner.startup_model_pull_pending = false;
    runner.first_frame_presented = false;

    runner.rebuild_scene_if_needed();

    assert!(
        !runner
            .frame_cache
            .text_runs
            .iter()
            .any(|run| run.text.contains("Starting interface"))
    );
}

#[test]
fn startup_defaults_match_platform_startup_strategy() {
    let runner = NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    assert!(!runner.startup_window_visible);
    assert!(!runner.startup_model_pull_pending);
    assert!(!NativeVelloRunner::<RecordingBridge>::startup_should_defer_first_model_pull());
}

#[test]
fn hidden_startup_arms_reveal_deadline_before_first_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.arm_startup_reveal_deadline(Instant::now());

    assert!(runner.startup_reveal_deadline.is_some());
}

#[test]
fn complete_first_present_schedules_deferred_model_pull() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;
    assert!(runner.startup_model_pull_pending);
    assert!(!runner.startup_deferred_model_refresh_pending);
    assert!(!runner.first_frame_presented);
    assert!(!runner.startup_window_visible);

    runner.complete_first_present();

    assert!(runner.first_frame_presented);
    assert!(!runner.startup_model_pull_pending);
    assert!(runner.startup_deferred_model_refresh_pending);
    assert!(!runner.startup_window_visible);
    assert!(runner.frame_state.take_model());
    assert!(!runner.frame_state.take_scene());
    assert!(runner.frame_state.take_state_overlay());
    assert!(runner.frame_state.take_motion_overlay());
}

#[test]
fn startup_window_reveals_after_deferred_model_refresh_present() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;

    runner.complete_first_present();
    assert!(!runner.startup_window_visible);

    runner.startup_deferred_model_refresh_pending = false;
    runner.complete_first_present();

    assert!(runner.startup_window_visible);
}

#[test]
fn startup_window_reveals_on_first_present_without_deferred_pull() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;

    runner.complete_first_present();

    assert!(runner.startup_window_visible);
    assert!(runner.first_frame_presented);
    assert!(!runner.startup_deferred_model_refresh_pending);
    assert_eq!(runner.startup_reveal_deadline, None);
    assert!(runner.startup_timing.did_emit_summary());
}

#[test]
fn startup_window_force_reveal_fallback_unblocks_hidden_stalls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;
    runner.complete_first_present();

    assert!(runner.startup_deferred_model_refresh_pending);
    assert!(!runner.startup_window_visible);
    runner.startup_reveal_deadline = Some(Instant::now() - Duration::from_millis(1));

    runner.maybe_force_reveal_startup_window_on_stall(Instant::now());

    assert!(runner.startup_window_visible);
    assert!(runner.startup_deferred_model_refresh_pending);
    assert_eq!(runner.startup_reveal_deadline, None);
}

#[test]
fn startup_window_reveals_after_first_scene_when_deferred_pull_is_disabled() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;
    runner.startup_deferred_model_refresh_pending = false;
    runner.first_frame_presented = false;
    runner.startup_window_visible = false;

    runner.maybe_reveal_startup_window_after_first_scene_ready();

    assert!(runner.startup_window_visible);
    assert_eq!(runner.startup_reveal_deadline, None);
}

#[test]
fn startup_window_force_reveal_fallback_unblocks_pre_first_present_stalls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_window_visible = false;
    runner.startup_reveal_deadline = Some(Instant::now() - Duration::from_millis(1));

    runner.maybe_force_reveal_startup_window_on_stall(Instant::now());

    assert!(runner.startup_window_visible);
    assert_eq!(runner.startup_reveal_deadline, None);
}

#[test]
fn deferred_startup_summary_waits_for_follow_up_refresh() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_timing.mark_init_started();
    runner.startup_timing.mark_window_created();
    runner.startup_timing.mark_surface_ready();
    runner.startup_timing.mark_renderer_ready();
    runner.startup_timing.mark_first_scene_ready();
    runner.startup_model_pull_pending = true;

    runner.complete_first_present();

    assert!(!runner.startup_timing.did_emit_summary());

    runner.startup_deferred_model_refresh_pending = true;
    runner.startup_deferred_model_refresh_pending = false;
    runner.startup_reveal_deadline = None;
    runner.startup_timing.mark_deferred_model_refresh_done();
    runner.startup_timing.maybe_emit_summary();

    assert!(runner.startup_timing.did_emit_summary());
}
