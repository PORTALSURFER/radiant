use super::*;

#[test]
/// Waveform drag-mode mapping should preserve the initial action intent.
fn waveform_drag_mode_maps_from_waveform_actions() {
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SeekWaveform {
            position_milli: 250
        }),
        Some(WaveformPointerDragMode::Seek)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformCursor {
            position_milli: 250
        }),
        Some(WaveformPointerDragMode::Cursor)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformSelectionRange {
            start_micros: milli(125),
            end_micros: milli(250),
            preserve_view_edge: false,
        }),
        Some(WaveformPointerDragMode::Selection {
            anchor_micros: milli(125)
        })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: milli(125),
            end_micros: milli(250),
        }),
        Some(WaveformPointerDragMode::SelectionSmartScale {
            anchor_micros: milli(125)
        })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::BeginWaveformSelectionShift {
            pointer_micros: milli(400),
            start_micros: milli(125),
            end_micros: milli(250),
        }),
        Some(WaveformPointerDragMode::SelectionShift {
            pointer_micros: milli(400),
            start_micros: milli(125),
            end_micros: milli(250),
        })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(90),
            end_micros: milli(320),
            preserve_view_edge: false,
        }),
        Some(WaveformPointerDragMode::EditSelection {
            anchor_micros: milli(90)
        })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::BeginWaveformEditSelectionShift {
            pointer_micros: milli(410),
            start_micros: milli(90),
            end_micros: milli(320),
        }),
        Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: milli(410),
            start_micros: milli(90),
            end_micros: milli(320),
        })
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeInEnd {
            position_micros: milli(200),
        }),
        Some(WaveformPointerDragMode::EditFadeInEnd)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeInMuteStart {
            position_micros: milli(150),
        }),
        Some(WaveformPointerDragMode::EditFadeInMuteStart)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeInCurve { curve_milli: 650 }),
        Some(WaveformPointerDragMode::EditFadeInCurve)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeOutStart {
            position_micros: milli(800),
        }),
        Some(WaveformPointerDragMode::EditFadeOutStart)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeOutMuteEnd {
            position_micros: milli(850),
        }),
        Some(WaveformPointerDragMode::EditFadeOutMuteEnd)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::SetWaveformEditFadeOutCurve { curve_milli: 350 }),
        Some(WaveformPointerDragMode::EditFadeOutCurve)
    );
    assert_eq!(
        waveform_drag_mode_for_action(&UiAction::ToggleTransport),
        None
    );
    assert!(waveform_drag_mode_is_edit_fade(
        WaveformPointerDragMode::EditFadeOutMuteEnd
    ));
    assert!(!waveform_drag_mode_is_edit_fade(
        WaveformPointerDragMode::Selection {
            anchor_micros: milli(250)
        }
    ));
    assert!(!waveform_drag_mode_is_edit_fade(
        WaveformPointerDragMode::SelectionSmartScale {
            anchor_micros: milli(250)
        }
    ));
    assert!(!waveform_drag_mode_is_edit_fade(
        WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: milli(400),
            start_micros: milli(200),
            end_micros: milli(600),
        }
    ));
}

#[test]
fn waveform_press_action_emit_policy_defers_mark_gestures() {
    assert!(waveform_press_action_emits_immediately(
        &UiAction::SeekWaveform {
            position_milli: 250,
        }
    ));
    assert!(waveform_press_action_emits_immediately(
        &UiAction::SetWaveformCursor {
            position_milli: 250,
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformSelectionRange {
            start_micros: milli(125),
            end_micros: milli(250),
            preserve_view_edge: false,
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: milli(125),
            end_micros: milli(250),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::BeginWaveformSelectionShift {
            pointer_micros: milli(300),
            start_micros: milli(125),
            end_micros: milli(250),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(90),
            end_micros: milli(320),
            preserve_view_edge: false,
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::BeginWaveformEditSelectionShift {
            pointer_micros: milli(310),
            start_micros: milli(90),
            end_micros: milli(320),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeInEnd {
            position_micros: milli(200),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeInMuteStart {
            position_micros: milli(150),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeInCurve { curve_milli: 650 }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeOutStart {
            position_micros: milli(800),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeOutMuteEnd {
            position_micros: milli(850),
        }
    ));
    assert!(!waveform_press_action_emits_immediately(
        &UiAction::SetWaveformEditFadeOutCurve { curve_milli: 350 }
    ));
}

#[test]
fn handle_pointer_press_action_arms_waveform_selection_without_emitting() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::SetWaveformSelectionRange {
            start_micros: milli(250),
            end_micros: milli(250),
            preserve_view_edge: false,
        },
        false,
    );

    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());
    assert_eq!(
        runner.waveform_drag_mode,
        Some(WaveformPointerDragMode::Selection {
            anchor_micros: milli(250)
        })
    );
}

#[test]
fn handle_pointer_press_action_arms_waveform_edit_selection_without_emitting() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::SetWaveformEditSelectionRange {
            start_micros: milli(400),
            end_micros: milli(400),
            preserve_view_edge: false,
        },
        false,
    );

    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());
    assert_eq!(
        runner.waveform_drag_mode,
        Some(WaveformPointerDragMode::EditSelection {
            anchor_micros: milli(400)
        })
    );
}

#[test]
fn handle_pointer_press_action_arms_selection_shift_without_emitting() {
    let mut runner =
        NativeVelloRunner::new(NativeRunOptions::default(), RecordingBridge::default());

    let emitted = runner.handle_pointer_press_action(
        UiAction::BeginWaveformSelectionShift {
            pointer_micros: milli(400),
            start_micros: milli(200),
            end_micros: milli(600),
        },
        false,
    );

    assert!(!emitted);
    assert!(runner.bridge.actions.is_empty());
    assert_eq!(
        runner.waveform_drag_mode,
        Some(WaveformPointerDragMode::SelectionShift {
            pointer_micros: milli(400),
            start_micros: milli(200),
            end_micros: milli(600),
        })
    );
}
