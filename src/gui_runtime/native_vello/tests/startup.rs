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
fn startup_layout_dirty_marks_full_layout_rebuild_contract() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.frame_state.mark_layout_dirty();

    assert!(runner.frame_state.layout_invalidation.full_rebuild);
    assert_eq!(
        runner.frame_state.layout_invalidation.dirty_segments.bits(),
        DirtySegments::STATUS_BAR
            | DirtySegments::BROWSER_FRAME
            | DirtySegments::BROWSER_ROWS_WINDOW
            | DirtySegments::MAP_PANEL
            | DirtySegments::WAVEFORM_OVERLAY
            | DirtySegments::GLOBAL_STATIC
    );
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
    assert!(runner.startup_model_pull_pending);
    assert!(NativeVelloRunner::<RecordingBridge>::startup_should_defer_first_model_pull());
}

#[test]
fn pre_renderer_startup_reveal_marks_window_visible_and_requests_redraw() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.maybe_reveal_startup_window_before_renderer_ready();

    #[cfg(target_os = "windows")]
    {
        assert!(runner.startup_window_visible);
        assert_eq!(runner.redraw_requested, runner.window.is_some());
    }
    #[cfg(not(target_os = "windows"))]
    {
        assert!(!runner.startup_window_visible);
        assert!(!runner.redraw_requested);
    }
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
    assert!(runner.startup_window_visible);
    assert_eq!(runner.startup_reveal_deadline, None);
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
    assert!(runner.startup_window_visible);

    runner.startup_deferred_model_refresh_pending = false;
    runner.complete_first_present();

    assert!(runner.startup_window_visible);
}

#[test]
fn startup_window_reveals_on_first_present_without_deferred_pull() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;
    runner.startup_timing.mark_init_started();
    runner.startup_timing.mark_window_created();
    runner.startup_timing.mark_surface_ready();
    runner.startup_timing.mark_renderer_ready();
    runner.startup_timing.mark_first_scene_ready();

    runner.complete_first_present();

    assert!(runner.startup_window_visible);
    assert!(runner.first_frame_presented);
    assert!(!runner.startup_deferred_model_refresh_pending);
    assert_eq!(runner.startup_reveal_deadline, None);
    assert!(runner.startup_timing.did_emit_summary());
}

#[test]
fn startup_summary_falls_back_to_first_present_when_window_reveal_is_untracked() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;
    runner.startup_timing.mark_init_started();
    runner.startup_timing.mark_window_created();
    runner.startup_timing.mark_surface_ready();
    runner.startup_timing.mark_renderer_ready();
    runner.startup_timing.mark_first_scene_ready();

    runner.complete_first_present();

    assert!(runner.startup_timing.did_emit_summary());
}

#[test]
fn startup_window_force_reveal_fallback_unblocks_hidden_stalls() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    assert!(!runner.startup_window_visible);
    runner.startup_reveal_deadline = Some(Instant::now() - Duration::from_millis(1));

    runner.maybe_force_reveal_startup_window_on_stall(Instant::now());

    assert!(runner.startup_window_visible);
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
fn startup_window_reveals_after_placeholder_scene_when_deferred_pull_is_enabled() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;
    runner.startup_deferred_model_refresh_pending = false;
    runner.first_frame_presented = false;
    runner.startup_window_visible = false;

    runner.maybe_reveal_startup_window_after_first_scene_ready();

    assert!(runner.startup_window_visible);
    assert_eq!(runner.startup_reveal_deadline, None);
    assert_eq!(runner.redraw_requested, runner.window.is_some());
}

#[test]
fn startup_window_reveals_on_first_present_even_with_deferred_refresh_pending() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = true;
    runner.startup_window_visible = false;

    runner.complete_first_present();

    assert!(runner.first_frame_presented);
    assert!(runner.startup_window_visible);
    assert!(runner.startup_deferred_model_refresh_pending);
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

    assert!(runner.startup_timing.did_emit_summary());
}

#[test]
fn startup_summary_emits_after_first_present_even_without_first_scene_marker() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.startup_model_pull_pending = false;
    runner.startup_timing.mark_init_started();
    runner.startup_timing.mark_window_created();
    runner.startup_timing.mark_surface_ready();
    runner.startup_timing.mark_renderer_ready();

    runner.complete_first_present();

    assert!(runner.startup_timing.did_emit_summary());
}

#[test]
fn startup_profile_failure_reason_is_explicit_before_first_present() {
    let mut timing = StartupTimingProfile::default();
    timing.mark_init_started();
    timing.mark_window_created();
    timing.mark_surface_ready();
    timing.mark_renderer_ready();
    timing.mark_first_scene_ready();

    assert_eq!(
        timing.failure_reason_for_test(),
        Some("startup_exited_before_first_present")
    );
}

#[test]
fn startup_profile_failure_reason_clears_after_summary_emits() {
    let mut timing = StartupTimingProfile::default();
    timing.mark_init_started();
    timing.mark_window_created();
    timing.mark_surface_ready();
    timing.mark_renderer_ready();
    timing.mark_first_scene_ready();
    timing.mark_first_presented();
    timing.maybe_emit_summary();

    assert_eq!(timing.failure_reason_for_test(), None);
}

#[test]
fn startup_profile_exports_complete_artifact_after_first_present() {
    let mut timing = StartupTimingProfile::default();
    timing.mark_init_started();
    timing.mark_window_created();
    timing.mark_surface_ready();
    timing.mark_renderer_started();
    timing.mark_renderer_ready();
    timing.mark_first_scene_ready();
    timing.mark_first_redraw_started();
    timing.mark_first_presented();
    timing.maybe_emit_summary();

    let artifact = timing.export_artifact().expect("startup timing artifact");
    assert_eq!(artifact.status, "complete");
    assert_eq!(artifact.failure_reason, None);
    assert!(artifact.first_present_ms.is_some());
    assert!(artifact.window_revealed_ms.is_some());
}

#[test]
fn startup_profile_exports_incomplete_artifact_before_first_present() {
    let mut timing = StartupTimingProfile::default();
    timing.mark_init_started();
    timing.mark_window_created();
    timing.mark_surface_ready();
    timing.mark_renderer_ready();

    let artifact = timing.export_artifact().expect("startup timing artifact");
    assert_eq!(artifact.status, "incomplete");
    assert_eq!(
        artifact.failure_reason.as_deref(),
        Some("startup_exited_before_first_present")
    );
    assert!(artifact.window_create_ms.is_some());
    assert_eq!(artifact.first_present_ms, None);
}
