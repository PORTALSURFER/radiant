use super::super::*;

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

    assert!(try_mark_repaint_event_pending(&pending));
    assert!(pending.load(Ordering::Acquire));
    assert!(!try_mark_repaint_event_pending(&pending));
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
