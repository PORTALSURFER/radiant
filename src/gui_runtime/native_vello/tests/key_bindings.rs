use super::*;

#[test]
fn key_repeat_is_limited_to_plain_browser_arrow_navigation() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    assert!(runner.allows_key_repeat(KeyCode::ArrowUp));
    assert!(runner.allows_key_repeat(KeyCode::ArrowDown));
    assert!(!runner.allows_key_repeat(KeyCode::Enter));

    runner.modifiers = ModifiersState::SHIFT;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowDown));

    runner.modifiers = ModifiersState::default();
    runner.text_input_target = TextInputTarget::BrowserSearch;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowDown));
}

#[test]
fn key_repeat_allows_shifted_arrow_steps_for_waveform_bpm_input() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::WaveformBpm;
    runner.modifiers = ModifiersState::SHIFT;

    assert!(runner.allows_key_repeat(KeyCode::ArrowUp));
    assert!(runner.allows_key_repeat(KeyCode::ArrowDown));

    runner.modifiers = ModifiersState::SHIFT | ModifiersState::CONTROL;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowUp));
}

#[test]
fn waveform_bpm_input_shift_arrow_steps_by_tenth() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::WaveformBpm;
    runner.waveform_bpm_input_buffer = Some(String::from("120.0"));

    assert!(runner.step_waveform_bpm_input(1));
    assert_eq!(runner.waveform_bpm_input_buffer.as_deref(), Some("120.1"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformBpmValue { value_tenths: 1201 }]
    );

    runner.bridge.actions.clear();
    assert!(runner.step_waveform_bpm_input(-10));
    assert_eq!(runner.waveform_bpm_input_buffer.as_deref(), Some("119.1"));
    assert_eq!(
        runner.bridge.actions,
        vec![UiAction::SetWaveformBpmValue { value_tenths: 1191 }]
    );
}

#[test]
fn g_prefix_routes_section_focus_commands() {
    assert_eq!(
        action_from_g_prefix_for_tests(KeyCode::G, KeyCode::W),
        Some(UiAction::FocusWaveformPanel)
    );
    assert_eq!(
        action_from_g_prefix_for_tests(KeyCode::G, KeyCode::B),
        Some(UiAction::FocusBrowserPanel)
    );
    assert_eq!(
        action_from_g_prefix_for_tests(KeyCode::G, KeyCode::T),
        Some(UiAction::FocusFolderPanel)
    );
    assert_eq!(
        action_from_g_prefix_for_tests(KeyCode::G, KeyCode::S),
        Some(UiAction::FocusSourcesPanel)
    );
    assert_eq!(
        action_from_g_prefix_for_tests(KeyCode::G, KeyCode::D),
        None
    );
}

#[test]
fn explicit_focus_is_required_for_scope_specific_hotkeys() {
    let none = AppModel::default();
    assert_eq!(action_from_key(KeyCode::N, ModifiersState::default(), &none), None);
    assert_eq!(action_from_key(KeyCode::D, ModifiersState::default(), &none), None);
    assert_eq!(action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &none), None);
    assert_eq!(action_from_key(KeyCode::W, ModifiersState::default(), &none), None);
    assert_eq!(action_from_key(KeyCode::S, ModifiersState::default(), &none), None);

    let browser = AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
    assert_eq!(
        action_from_key(KeyCode::N, ModifiersState::default(), &browser),
        Some(UiAction::NormalizeFocusedBrowserSample)
    );
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &browser),
        Some(UiAction::DeleteBrowserSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::R, ModifiersState::default(), &browser),
        Some(UiAction::StartBrowserRename)
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &browser),
        Some(UiAction::MoveBrowserFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowDown, ModifiersState::SHIFT, &browser),
        Some(UiAction::ExtendBrowserSelectionFromFocus { delta: 1 })
    );
    assert_eq!(
        action_from_key(
            KeyCode::A,
            ModifiersState::CONTROL,
            &browser,
        ),
        Some(UiAction::SelectAllBrowserRows)
    );

    let folders = AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    };
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &folders),
        Some(UiAction::DeleteFocusedFolder)
    );
    assert_eq!(
        action_from_key(KeyCode::R, ModifiersState::default(), &folders),
        Some(UiAction::StartFolderRename)
    );
    assert_eq!(
        action_from_key(KeyCode::Quote, ModifiersState::default(), &folders),
        Some(UiAction::FocusFolderSearch)
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowDown, ModifiersState::default(), &folders),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
}

#[test]
fn sources_list_and_waveform_hotkeys_resolve_by_focus_mode() {
    let sources = AppModel {
        focus_context: crate::app::FocusContextModel::SourcesList,
        ..AppModel::default()
    };
    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &sources),
        Some(UiAction::MoveSourceFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowDown, ModifiersState::default(), &sources),
        Some(UiAction::MoveSourceFocus { delta: 1 })
    );
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &sources),
        Some(UiAction::RemoveFocusedSourceRow)
    );
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::SHIFT, &sources),
        Some(UiAction::RemoveDeadLinksForFocusedSourceRow)
    );
    assert_eq!(
        action_from_key(KeyCode::R, ModifiersState::default(), &sources),
        Some(UiAction::ReloadFocusedSourceRow)
    );

    let waveform = AppModel {
        focus_context: crate::app::FocusContextModel::Waveform,
        ..AppModel::default()
    };
    assert_eq!(
        action_from_key(KeyCode::B, ModifiersState::default(), &waveform),
        Some(UiAction::ToggleBpmSnap)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &waveform),
        Some(UiAction::CropWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::SHIFT, &waveform),
        Some(UiAction::CropWaveformSelectionToNewSample)
    );
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &waveform),
        Some(UiAction::DeleteLoadedWaveformSample)
    );
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::SHIFT, &waveform),
        Some(UiAction::DeleteSelectedSliceMarkers)
    );
    assert_eq!(
        action_from_key(KeyCode::M, ModifiersState::default(), &waveform),
        Some(UiAction::MuteWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::N, ModifiersState::default(), &waveform),
        Some(UiAction::NormalizeWaveformSelectionOrSample)
    );
    assert_eq!(
        action_from_key(KeyCode::S, ModifiersState::default(), &waveform),
        Some(UiAction::AlignWaveformStartToMarker)
    );
    assert_eq!(
        action_from_key(KeyCode::T, ModifiersState::default(), &waveform),
        Some(UiAction::TrimWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowLeft, ModifiersState::default(), &waveform),
        Some(UiAction::SlideWaveformSelection {
            delta: -1,
            fine: false,
        })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowRight, ModifiersState::SHIFT, &waveform),
        Some(UiAction::SlideWaveformSelection {
            delta: 1,
            fine: true,
        })
    );
    assert_eq!(
        action_from_key(KeyCode::Slash, ModifiersState::default(), &waveform),
        Some(UiAction::FadeWaveformSelectionRightToLeft)
    );
    assert_eq!(
        action_from_key(KeyCode::Backslash, ModifiersState::default(), &waveform),
        Some(UiAction::FadeWaveformSelectionLeftToRight)
    );
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &waveform),
        Some(UiAction::SaveWaveformSelectionToBrowser)
    );
}

#[test]
fn key_bindings_respect_progress_cancelability_and_playback_shortcuts() {
    let mut model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::P, ModifiersState::default(), &model),
        None
    );

    model.progress_overlay.cancelable = true;
    assert_eq!(
        action_from_key(KeyCode::P, ModifiersState::default(), &model),
        Some(UiAction::CancelProgress)
    );

    assert_eq!(
        action_from_key(KeyCode::Space, ModifiersState::default(), &model),
        Some(UiAction::PlayFromStart)
    );
    assert_eq!(
        action_from_key(KeyCode::Space, ModifiersState::CONTROL, &model),
        Some(UiAction::PlayFromCurrentPlayhead)
    );
}

#[test]
fn clicking_browser_search_field_focuses_text_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel::default();
    let search_field = shell_state
        .browser_search_field_rect(&layout, &model)
        .expect("browser search field should be present");
    let point = Point::new(
        (search_field.min.x + search_field.max.x) * 0.5,
        (search_field.min.y + search_field.max.y) * 0.5,
    );

    assert_eq!(
        action_from_pointer(
            &layout,
            &model,
            &mut shell_state,
            point,
            ModifiersState::default(),
        ),
        Some(UiAction::FocusBrowserSearch)
    );
}

#[test]
fn browser_search_editor_supports_copy_paste_and_delete_selection() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::BrowserSearch;
    runner.text_input_buffer = Some(String::from("kick"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("kick"));

    assert!(runner.select_all_text());
    assert!(runner.copy_selected_text());
    assert_eq!(runner.clipboard_fallback_text, "kick");

    assert!(runner.write_clipboard_text("snare"));
    assert!(runner.paste_text());
    assert_eq!(runner.text_input_buffer.as_deref(), Some("snare"));
    assert_eq!(
        runner.bridge.actions.last(),
        Some(&UiAction::SetBrowserSearch {
            query: String::from("snare"),
        })
    );

    assert!(runner.select_all_text());
    assert!(runner.delete_text_forward());
    assert_eq!(runner.text_input_buffer.as_deref(), Some(""));
}

#[test]
fn waveform_bpm_editor_supports_copy_paste_and_delete_selection() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.text_input_target = TextInputTarget::WaveformBpm;
    runner.waveform_bpm_input_buffer = Some(String::from("120.0"));
    runner.text_editor_state = Some(SingleLineTextEditorState::collapsed_at_end("120.0"));

    assert!(runner.select_all_text());
    assert!(runner.copy_selected_text());
    assert_eq!(runner.clipboard_fallback_text, "120.0");

    assert!(runner.write_clipboard_text("98.5"));
    assert!(runner.paste_text());
    assert_eq!(runner.waveform_bpm_input_buffer.as_deref(), Some("98.5"));
    assert_eq!(
        runner.bridge.actions.last(),
        Some(&UiAction::SetWaveformBpmValue { value_tenths: 985 })
    );

    assert!(runner.select_all_text());
    assert!(runner.delete_text_forward());
    assert_eq!(runner.waveform_bpm_input_buffer.as_deref(), Some(""));
}
