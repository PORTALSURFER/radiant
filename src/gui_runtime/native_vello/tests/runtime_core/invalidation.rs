use super::super::*;
use crate::gui::native_shell::{ShellLayoutDirtyKind, ShellLayoutTreeKind};

fn install_test_layout(runner: &mut NativeVelloRunner<RecordingBridge>) {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);
    runner.shell_layout = Some(Arc::new(layout));
    runner.style_cache = Some(style);
    runner.motion_model = Some(NativeMotionModel::from_app_model(&runner.model));
}

fn install_clean_test_layout(runner: &mut NativeVelloRunner<CleanBridge>) {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let style = StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale);
    runner.shell_layout = Some(Arc::new(layout));
    runner.style_cache = Some(style);
    runner.motion_model = Some(NativeMotionModel::from_app_model(&runner.model));
}

#[derive(Default)]
struct CleanBridge {
    model: Arc<AppModel>,
    dirty_segments: DirtySegments,
}

impl NativeAppBridge for CleanBridge {
    fn project_model(&mut self) -> Arc<AppModel> {
        Arc::clone(&self.model)
    }

    fn project_motion_model(&mut self) -> Option<NativeMotionModel> {
        Some(NativeMotionModel::from_app_model(&self.model))
    }

    fn take_dirty_segments(&mut self) -> DirtySegments {
        let dirty_segments = self.dirty_segments;
        self.dirty_segments = DirtySegments::empty();
        dirty_segments
    }
}

#[test]
fn action_scope_classification_routes_waveform_actions_by_cost() {
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SeekWaveform {
            position_milli: 420,
        }),
        RuntimeInvalidationScope::OverlayMotionOnly
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: None,
        }),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::SetWaveformViewCenter {
                center_micros: 420_000,
                center_nanos: None,
            }
        ),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::DetectWaveformSilenceSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::DetectWaveformExactDuplicateSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::CleanWaveformExactDuplicateSlices
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::StartWaveformSelectionDrag {
                pointer_x: 320,
                pointer_y: 240,
            }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::PlayFromStart),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::PlayFromCurrentPlayhead
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
}

#[test]
fn action_scope_classification_defaults_to_static_and_overlays_for_non_waveform_actions() {
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetBrowserSearch {
            query: String::from("kick"),
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetPromptInput {
            value: String::from("rename-me"),
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::MoveBrowserFocus {
            delta: 1
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::FocusBrowserRow {
            visible_row: 12
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetBrowserViewStart {
            visible_row: 4
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ToggleRandomNavigationMode
        ),
        RuntimeInvalidationScope::StaticAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SetVolume {
            value_milli: 250
        }),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::CommitVolumeSetting),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::FinishWaveformEditFadeDrag
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ToggleWaveformSliceSelection { index: 0 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::MoveWaveformSliceFocus { delta: 1 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::StartNewFolder),
        RuntimeInvalidationScope::StaticAndOverlays
    );
}

#[test]
fn browser_navigation_selection_actions_use_model_overlay_scope() {
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ToggleBrowserRowSelection { visible_row: 7 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(
            &UiAction::ExtendBrowserSelectionToRow { visible_row: 9 }
        ),
        RuntimeInvalidationScope::ModelAndOverlays
    );
    assert_eq!(
        NativeVelloRunner::<PreviewBridge>::classify_action_scope(&UiAction::SelectAllBrowserRows),
        RuntimeInvalidationScope::ModelAndOverlays
    );
}

#[test]
fn repaint_event_pending_gate_coalesces_duplicate_requests() {
    let pending = AtomicBool::new(false);

    assert!(super::super::super::legacy_shell_runtime::try_mark_repaint_event_pending(&pending));
    assert!(pending.load(Ordering::Acquire));
    assert!(!super::super::super::legacy_shell_runtime::try_mark_repaint_event_pending(&pending));
}

#[test]
fn model_overlay_dirty_does_not_force_static_scene_rebuild() {
    let mut state = NativeVelloFrameState::default();
    state.mark_model_overlay_dirty();
    assert!(state.take_model());
    assert!(!state.take_scene());
    assert!(state.take_state_overlay());
    assert!(state.take_motion_overlay());
}

#[test]
fn layout_dirty_only_requests_layout_and_static_scene_work() {
    let mut state = NativeVelloFrameState::default();
    state.mark_layout_dirty();

    assert!(state.take_layout_invalidation().is_pending());
    assert!(state.take_scene());
    assert!(!state.take_model());
    assert!(!state.take_state_overlay());
    assert!(!state.take_motion_overlay());
}

#[test]
fn model_dirty_only_requests_model_and_static_scene_work() {
    let mut state = NativeVelloFrameState::default();
    state.mark_model_dirty();

    assert!(!state.take_layout_invalidation().is_pending());
    assert!(state.take_model());
    assert!(state.take_scene());
    assert!(!state.take_state_overlay());
    assert!(!state.take_motion_overlay());
}

#[test]
fn resolve_static_rebuild_skips_static_for_model_overlay_when_bridge_clean() {
    let dirty = DirtySegments::empty();
    assert!(!resolve_static_rebuild(true, false, dirty));
}

#[test]
fn resolve_static_rebuild_keeps_explicit_static_invalidation() {
    let dirty = DirtySegments::empty();
    assert!(resolve_static_rebuild(true, true, dirty));
}

#[test]
fn resolve_static_rebuild_honors_bridge_static_dirty_segments() {
    let dirty = DirtySegments::from_bits(DirtySegments::STATUS_BAR);
    assert!(resolve_static_rebuild(true, false, dirty));
}

#[test]
fn static_rebuild_from_dirty_mask_requires_model_refresh_without_explicit_request() {
    let dirty = DirtySegments::from_bits(DirtySegments::STATUS_BAR);
    assert!(static_rebuild_from_dirty_mask(true, false, dirty));
    assert!(!static_rebuild_from_dirty_mask(false, false, dirty));
    assert!(!static_rebuild_from_dirty_mask(true, true, dirty));
}

#[test]
fn partial_layout_invalidation_marks_only_affected_static_segments() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.apply_invalidation_scope(RuntimeInvalidationScope::LayoutSubtreeAndAll(
        RuntimeLayoutSubtreeInvalidation::new(
            ShellLayoutTreeKind::BrowserBands,
            crate::gui::native_shell::BROWSER_BANDS_ROOT_ID,
            ShellLayoutDirtyKind::Measure,
        ),
    ));

    assert!(!runner.frame_state.layout_invalidation.full_rebuild);
    assert_eq!(
        runner.frame_state.layout_invalidation.dirty_segments.bits(),
        DirtySegments::BROWSER_FRAME
            | DirtySegments::BROWSER_ROWS_WINDOW
            | DirtySegments::MAP_PANEL
    );
    assert!(runner.frame_state.scene_dirty);
    assert!(runner.frame_state.state_overlay_dirty);
    assert!(runner.frame_state.motion_overlay_dirty);
    assert!(runner.frame_state.model_dirty);
}

#[test]
fn full_layout_invalidation_keeps_full_rebuild_escape_hatch() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    runner.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);

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
fn frame_result_marks_layout_and_static_rebuilds_for_layout_invalidations() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    install_test_layout(&mut runner);

    runner.apply_invalidation_scope(RuntimeInvalidationScope::LayoutSubtreeAndAll(
        RuntimeLayoutSubtreeInvalidation::new(
            ShellLayoutTreeKind::BrowserBands,
            crate::gui::native_shell::BROWSER_BANDS_ROOT_ID,
            ShellLayoutDirtyKind::Measure,
        ),
    ));

    let (redrew, result) = runner.rebuild_scene_for_redraw(false, 0.0);

    assert!(redrew);
    assert!(result.layout_rebuild);
    assert!(result.static_rebuild);
    assert!(result.state_overlay_rebuild);
    assert!(result.motion_overlay_rebuild);
}

#[test]
fn frame_result_marks_overlay_only_redraws_without_static_or_layout_rebuilds() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    install_test_layout(&mut runner);
    runner.frame_state.scene_dirty = true;
    let _ = runner.rebuild_scene_for_redraw(false, 0.0);
    runner.frame_state.scene_dirty = false;
    runner.frame_state.model_dirty = false;
    runner.frame_state.state_overlay_dirty = false;
    runner.frame_state.motion_overlay_dirty = true;

    let (redrew, result) = runner.rebuild_scene_for_redraw(false, 0.0);

    assert!(redrew);
    assert!(!result.layout_rebuild);
    assert!(!result.static_rebuild);
    assert!(!result.state_overlay_rebuild);
    assert!(result.motion_overlay_rebuild);
}

#[test]
fn static_scene_dirty_does_not_rebuild_warm_overlay_caches() {
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), CleanBridge::default());
    install_clean_test_layout(&mut runner);
    runner.frame_state.scene_dirty = true;
    runner.frame_state.state_overlay_dirty = true;
    runner.frame_state.motion_overlay_dirty = true;
    let _ = runner.rebuild_scene_for_redraw(false, 0.0);

    runner.frame_state.scene_dirty = true;
    let (redrew, result) = runner.rebuild_scene_for_redraw(false, 0.0);

    assert!(redrew);
    assert!(!result.layout_rebuild);
    assert!(result.static_rebuild);
    assert!(!result.state_overlay_rebuild);
    assert!(!result.motion_overlay_rebuild);
}

#[test]
fn static_model_dirty_does_not_force_state_overlay_rebuild_when_bridge_reports_no_overlay_delta() {
    let mut runner = NativeVelloRunner::new(NativeRunOptions::default(), CleanBridge::default());
    install_clean_test_layout(&mut runner);
    runner.apply_invalidation_scope(RuntimeInvalidationScope::StaticAndOverlays);
    let _ = runner.rebuild_scene_for_redraw(false, 0.0);

    runner.frame_state.mark_model_dirty();
    let (redrew, result) = runner.rebuild_scene_for_redraw(false, 0.0);

    assert!(redrew);
    assert!(!result.layout_rebuild);
    assert!(result.static_rebuild);
    assert!(!result.state_overlay_rebuild);
}
