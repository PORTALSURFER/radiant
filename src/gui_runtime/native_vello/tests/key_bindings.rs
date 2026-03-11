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
fn key_bindings_emit_rating_and_waveform_actions() {
    let mut model = AppModel::default();
    model.focus_context = crate::app::FocusContextModel::Waveform;
    assert_eq!(
        action_from_key(KeyCode::OpenBracket, ModifiersState::default(), &model),
        Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash
        })
    );
    assert_eq!(
        action_from_key(KeyCode::CloseBracket, ModifiersState::default(), &model),
        Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep
        })
    );
    assert_eq!(
        action_from_key(KeyCode::M, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveformToSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &model),
        Some(UiAction::CropWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::SHIFT, &model),
        Some(UiAction::CropWaveformSelectionToNewSample)
    );
    assert_eq!(
        action_from_key(KeyCode::T, ModifiersState::default(), &model),
        Some(UiAction::TrimWaveformSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::Slash, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveformFull)
    );
}

#[test]
fn key_bindings_emit_browser_actions() {
    let mut model = AppModel::default();
    model.focus_context = crate::app::FocusContextModel::SampleBrowser;
    assert_eq!(
        action_from_key(KeyCode::D, ModifiersState::default(), &model),
        Some(UiAction::DeleteBrowserSelection)
    );
    assert_eq!(
        action_from_key(KeyCode::I, ModifiersState::default(), &model),
        Some(UiAction::StartBrowserRename)
    );
    assert_eq!(
        action_from_key(KeyCode::N, ModifiersState::default(), &model),
        Some(UiAction::NormalizeFocusedBrowserSample)
    );
    assert_eq!(
        action_from_key(KeyCode::X, ModifiersState::default(), &model),
        Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash
        })
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &model),
        None
    );
    assert_eq!(
        action_from_key(KeyCode::T, ModifiersState::default(), &model),
        None
    );
}

#[test]
fn key_bindings_map_n_to_waveform_normalize_when_waveform_is_focused() {
    let mut model = AppModel::default();
    model.focus_context = crate::app::FocusContextModel::Waveform;
    assert_eq!(
        action_from_key(KeyCode::N, ModifiersState::default(), &model),
        Some(UiAction::NormalizeWaveformSelectionOrSample)
    );
}

#[test]
fn key_bindings_emit_folder_actions() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::B, ModifiersState::default(), &model),
        Some(UiAction::StartNewFolder)
    );
    assert_eq!(
        action_from_key(KeyCode::G, ModifiersState::default(), &model),
        Some(UiAction::DeleteFocusedFolder)
    );
    assert_eq!(
        action_from_key(KeyCode::Quote, ModifiersState::default(), &model),
        Some(UiAction::FocusFolderSearch)
    );
    assert_eq!(
        action_from_key(KeyCode::Z, ModifiersState::default(), &model),
        Some(UiAction::StartFolderRename)
    );
}

#[test]
fn key_bindings_map_z_to_waveform_zoom_when_waveform_is_focused() {
    let mut model = AppModel::default();
    model.focus_context = crate::app::FocusContextModel::Waveform;
    assert_eq!(
        action_from_key(KeyCode::Z, ModifiersState::default(), &model),
        Some(UiAction::ZoomWaveformToSelection)
    );
}

#[test]
fn key_bindings_map_u_to_undo_and_shift_u_to_redo() {
    let model = AppModel::default();

    assert_eq!(
        action_from_key(KeyCode::U, ModifiersState::default(), &model),
        Some(UiAction::Undo)
    );
    assert_eq!(
        action_from_key(KeyCode::U, ModifiersState::SHIFT, &model),
        Some(UiAction::Redo)
    );
}

#[test]
fn prompt_visible_routes_enter_and_cancel_keys() {
    let mut model = AppModel::default();
    model.confirm_prompt.visible = true;
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        Some(UiAction::ConfirmPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::default(), &model),
        Some(UiAction::CancelPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::W, ModifiersState::default(), &model),
        None
    );

    model.confirm_prompt.input_error = Some(String::from("Folder already exists"));
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        None
    );
}

#[test]
fn key_bindings_handle_selection_modifiers() {
    let model = AppModel::default();

    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &model),
        Some(UiAction::MoveBrowserFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::SHIFT, &model),
        Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(
            KeyCode::ArrowUp,
            ModifiersState::SHIFT | ModifiersState::CONTROL,
            &model
        ),
        Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(
            KeyCode::ArrowDown,
            ModifiersState::SHIFT | ModifiersState::SUPER,
            &model
        ),
        Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
    );
    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        Some(UiAction::CommitFocusedBrowserRow)
    );
}

#[test]
fn key_bindings_route_arrow_navigation_to_folder_tree_when_folder_focused() {
    let model = AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    };

    assert_eq!(
        action_from_key(KeyCode::ArrowUp, ModifiersState::default(), &model),
        Some(UiAction::MoveFolderFocus { delta: -1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowDown, ModifiersState::default(), &model),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
    assert_eq!(
        action_from_key(KeyCode::ArrowDown, ModifiersState::SHIFT, &model),
        Some(UiAction::MoveFolderFocus { delta: 1 })
    );
}

#[test]
fn key_bindings_route_enter_to_waveform_selection_export_when_waveform_focused() {
    let model = AppModel {
        focus_context: crate::app::FocusContextModel::Waveform,
        ..AppModel::default()
    };

    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::default(), &model),
        Some(UiAction::SaveWaveformSelectionToBrowser)
    );
}

#[test]
fn confirm_prompt_keys_ignore_other_shortcuts_when_visible() {
    let mut model = AppModel::default();
    model.confirm_prompt.visible = true;

    assert_eq!(
        action_from_key(KeyCode::Enter, ModifiersState::SHIFT, &model),
        Some(UiAction::ConfirmPrompt)
    );
    assert_eq!(
        action_from_key(KeyCode::C, ModifiersState::SUPER, &model),
        Some(UiAction::CancelPrompt)
    );
}

#[test]
fn key_bindings_respect_progress_cancelability() {
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

#[test]
fn space_key_maps_to_play_from_start() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::Space, ModifiersState::default(), &model),
        Some(UiAction::PlayFromStart)
    );
}

#[test]
fn ctrl_space_key_maps_to_play_from_current_playhead() {
    let model = AppModel::default();
    assert_eq!(
        action_from_key(KeyCode::Space, ModifiersState::CONTROL, &model),
        Some(UiAction::PlayFromCurrentPlayhead)
    );
}
