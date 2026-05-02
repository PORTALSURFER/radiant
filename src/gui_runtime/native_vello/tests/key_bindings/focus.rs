use super::*;

#[test]
fn g_prefix_routes_section_focus_commands() {
    let focus = AppModel::default();
    let first = action_from_key(
        KeyCode::G,
        ModifiersState::default(),
        &focus,
        None,
        default_hotkey_resolver,
    );
    assert_eq!(
        first.pending_chord,
        Some(crate::compat_app_contract::KeyPress::new(KeyCode::G))
    );
    assert!(first.action.is_none());

    let second = action_from_key(
        KeyCode::W,
        ModifiersState::default(),
        &focus,
        first.pending_chord,
        default_hotkey_resolver,
    );
    assert_eq!(second.action, Some(UiAction::FocusWaveformPanel));
}

#[test]
fn explicit_focus_is_required_for_scope_specific_hotkeys() {
    let none = AppModel::default();
    assert_eq!(
        resolved_action(KeyCode::N, ModifiersState::default(), &none),
        None
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &none),
        None
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &none),
        None
    );

    let browser = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::N, ModifiersState::default(), &browser),
        Some(UiAction::NormalizeFocusedContentItem)
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &browser),
        Some(UiAction::DeleteBrowserSelection)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &browser),
        Some(UiAction::MoveBrowserFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &browser),
        Some(UiAction::MoveBrowserFocus { delta: 1 })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &browser),
        Some(UiAction::ToggleFocusedBrowserRowSelection)
    );

    let folders = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::NavigationTree,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &folders),
        Some(UiAction::DeleteFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowLeft, ModifiersState::default(), &folders),
        Some(UiAction::CollapseFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::default(), &folders),
        Some(UiAction::ExpandFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &folders),
        Some(UiAction::ToggleFocusedFolderSelection)
    );

    let sources = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::NavigationList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::R, ModifiersState::default(), &sources),
        Some(UiAction::ReloadFocusedSourceRow)
    );
}

#[test]
fn plain_s_routes_by_focus_between_browser_similarity_and_waveform_start_alignment() {
    let browser = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::S, ModifiersState::default(), &browser),
        Some(UiAction::ToggleFindSimilarFocusedContent)
    );

    let waveform = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::Timeline,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::S, ModifiersState::default(), &waveform),
        Some(UiAction::AlignWaveformStartToMarker)
    );
}

#[test]
fn waveform_hotkeys_resolve_by_focus_mode() {
    let waveform = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::Timeline,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::Enter, ModifiersState::default(), &waveform),
        Some(UiAction::CommitWaveformEditFades)
    );
    assert_eq!(
        resolved_action(KeyCode::E, ModifiersState::default(), &waveform),
        Some(UiAction::SaveWaveformSelectionToBrowser)
    );
    assert_eq!(
        resolved_action(KeyCode::E, ModifiersState::SHIFT, &waveform),
        Some(UiAction::SaveWaveformSelectionToBrowserWithKeep2)
    );
    assert_eq!(
        resolved_action(KeyCode::B, ModifiersState::default(), &waveform),
        Some(UiAction::ToggleBpmSnap)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::SHIFT, &waveform),
        Some(UiAction::SlideWaveformSelection {
            delta: 1,
            fine: true,
        })
    );
    assert_eq!(
        resolved_action(KeyCode::X, ModifiersState::default(), &waveform),
        Some(UiAction::ZoomWaveformFull)
    );
}

#[test]
fn folder_arrow_hotkeys_still_resolve_when_search_query_exists_but_tree_has_focus() {
    let mut folders = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::NavigationTree,
        ..AppModel::default()
    };
    folders.sources.tree_search_query = String::from("dr");

    assert_eq!(
        resolved_action(KeyCode::ArrowLeft, ModifiersState::default(), &folders),
        Some(UiAction::CollapseFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowRight, ModifiersState::default(), &folders),
        Some(UiAction::ExpandFocusedFolder)
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowUp, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: -1 })
    );
    assert_eq!(
        resolved_action(KeyCode::ArrowDown, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
}

#[test]
fn key_bindings_respect_progress_cancelability_and_playback_shortcuts() {
    let mut model = AppModel::default();
    assert_eq!(
        resolved_action(KeyCode::P, ModifiersState::default(), &model),
        None
    );

    model.progress_overlay.cancelable = true;
    assert_eq!(
        resolved_action(KeyCode::P, ModifiersState::default(), &model),
        Some(UiAction::CancelProgress)
    );
    assert_eq!(
        resolved_action(KeyCode::Space, ModifiersState::default(), &model),
        Some(UiAction::PlayFromStart)
    );
}

#[test]
fn semicolon_routes_browser_mark_without_conflicting_with_waveform_shortcuts() {
    let browser = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::ContentList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::Semicolon, ModifiersState::default(), &browser),
        Some(UiAction::ToggleContentMark)
    );

    let waveform = AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::Timeline,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::Semicolon, ModifiersState::default(), &waveform),
        None
    );
}
