use super::*;

fn resolved_action(key: KeyCode, modifiers: ModifiersState, model: &AppModel) -> Option<UiAction> {
    action_from_key(key, modifiers, model, None).action
}

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
}

#[test]
fn g_prefix_routes_section_focus_commands() {
    let focus = AppModel::default();
    let first = action_from_key(KeyCode::G, ModifiersState::default(), &focus, None);
    assert_eq!(
        first.pending_chord,
        Some(crate::app::KeyPress::new(KeyCode::G))
    );
    assert!(first.action.is_none());

    let second = action_from_key(
        KeyCode::W,
        ModifiersState::default(),
        &focus,
        first.pending_chord,
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
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::N, ModifiersState::default(), &browser),
        Some(UiAction::NormalizeFocusedBrowserSample)
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &browser),
        Some(UiAction::DeleteBrowserSelection)
    );

    let folders = AppModel {
        focus_context: crate::app::FocusContextModel::SourceFolders,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::default(), &folders),
        Some(UiAction::DeleteFocusedFolder)
    );

    let sources = AppModel {
        focus_context: crate::app::FocusContextModel::SourcesList,
        ..AppModel::default()
    };
    assert_eq!(
        resolved_action(KeyCode::R, ModifiersState::default(), &sources),
        Some(UiAction::ReloadFocusedSourceRow)
    );
    assert_eq!(
        resolved_action(KeyCode::D, ModifiersState::SHIFT, &sources),
        Some(UiAction::RemoveDeadLinksForFocusedSourceRow)
    );
}

#[test]
fn waveform_hotkeys_resolve_by_focus_mode() {
    let waveform = AppModel {
        focus_context: crate::app::FocusContextModel::Waveform,
        ..AppModel::default()
    };
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
fn clicking_browser_search_field_focuses_text_input() {
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let mut shell_state = NativeShellState::new();
    let model = AppModel {
        focus_context: crate::app::FocusContextModel::SampleBrowser,
        ..AppModel::default()
    };
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
