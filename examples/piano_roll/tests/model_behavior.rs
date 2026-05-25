use super::*;

#[test]
fn piano_roll_tick_advances_synthetic_playhead_without_midi_or_dsp() {
    let mut state = PianoRollState::default();
    let initial = state.playhead_beat;

    state.tick();

    assert_eq!(state.frame, 1);
    assert!(state.playhead_beat > initial);
    assert_eq!(DATA_SOURCE_NOTE, "without_midi_or_dsp");
}

#[test]
fn piano_roll_viewport_zoom_and_pan_updates_visible_range() {
    let mut state = PianoRollState::default();

    state.apply_roll_message(PianoRollMessage::ZoomTime { factor: 0.5 });
    state.apply_roll_message(PianoRollMessage::PanViewport {
        beat_delta: 3.0,
        pitch_delta: 0,
    });
    state.apply_roll_message(PianoRollMessage::ZoomPitch { rows_delta: -8 });
    state.apply_roll_message(PianoRollMessage::PanViewport {
        beat_delta: 0.0,
        pitch_delta: 4,
    });

    assert_eq!(state.viewport.visible_beats, 8.0);
    assert_eq!(state.viewport.beat_start, 7.0);
    assert_eq!(state.viewport.visible_pitches, 16);
    assert_eq!(state.viewport.pitch_start, 56);
    assert!(state.status().contains("beats 7.0-15.0"));
}
