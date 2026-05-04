use super::*;

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
fn key_repeat_allows_alt_arrow_micro_slides_only_in_waveform_focus() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());
    runner.modifiers = ModifiersState::ALT;

    assert!(!runner.allows_key_repeat(KeyCode::ArrowLeft));
    assert!(!runner.allows_key_repeat(KeyCode::ArrowRight));

    runner.model = Arc::new(AppModel {
        focus_context: crate::compat_app_contract::FocusContextModel::Timeline,
        ..AppModel::default()
    });
    assert!(runner.allows_key_repeat(KeyCode::ArrowLeft));
    assert!(runner.allows_key_repeat(KeyCode::ArrowRight));
    assert!(!runner.allows_key_repeat(KeyCode::ArrowUp));

    runner.text_input_target = TextInputTarget::ContentSearch;
    assert!(!runner.allows_key_repeat(KeyCode::ArrowLeft));
    assert!(!runner.allows_key_repeat(KeyCode::ArrowRight));
}
