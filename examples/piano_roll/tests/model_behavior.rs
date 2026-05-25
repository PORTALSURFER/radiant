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

#[test]
fn piano_roll_undo_redo_restores_registered_note_edits() {
    let mut state = PianoRollState::default();
    let initial_notes = state.notes.clone();

    update(
        &mut state,
        AppMessage::Roll(PianoRollMessage::CreateNote {
            pitch: 60,
            start_beat: 6.0,
            length_beats: 1.0,
        }),
    );

    assert_ne!(state.notes, initial_notes);
    assert_eq!(state.history.undo_len(), 1);
    update(&mut state, AppMessage::Undo);
    assert_eq!(state.notes, initial_notes);
    assert_eq!(state.history.redo_len(), 1);

    update(&mut state, AppMessage::Redo);
    assert_ne!(state.notes, initial_notes);
    assert_eq!(state.history.undo_len(), 1);
}

#[test]
fn piano_roll_undo_redo_shortcuts_route_through_radiant_history_support() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::Z),
        None,
        FocusSurface::None
    ));
    assert_eq!(
        status_text(&runtime),
        initial_status,
        "empty undo history should be handled without changing state"
    );

    runtime.dispatch_message(AppMessage::Roll(PianoRollMessage::ZoomTime { factor: 0.5 }));
    assert_ne!(status_text(&runtime), initial_status);

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::Z),
        None,
        FocusSurface::None
    ));
    assert_eq!(status_text(&runtime), initial_status);

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::Y),
        None,
        FocusSurface::None
    ));
    assert_ne!(status_text(&runtime), initial_status);
}

#[test]
fn piano_roll_resize_allows_notes_longer_than_four_beats() {
    let mut state = PianoRollState::default();

    state.apply_roll_message(PianoRollMessage::ResizeNote {
        id: 2,
        start_beat: 1.0,
        length_beats: 8.0,
    });

    let note = state
        .notes
        .iter()
        .find(|note| note.id == 2)
        .expect("resized note should still exist");
    assert_eq!(note.start_beat, 1.0);
    assert_eq!(note.length_beats, 8.0);
}

#[test]
fn piano_roll_velocity_updates_coalesce_into_one_undo_step() {
    let mut state = PianoRollState::default();
    let initial = state.snapshot();

    update(
        &mut state,
        AppMessage::Roll(PianoRollMessage::SetVelocities {
            velocities: vec![(2, 0.2)],
        }),
    );
    update(
        &mut state,
        AppMessage::Roll(PianoRollMessage::SetVelocities {
            velocities: vec![(2, 0.4)],
        }),
    );

    assert_eq!(state.history.undo_len(), 1);
    assert!(
        state
            .notes
            .iter()
            .any(|note| { note.id == 2 && (note.velocity - 0.4).abs() < f32::EPSILON })
    );
    update(&mut state, AppMessage::Undo);
    assert_eq!(state.snapshot(), initial);
}

#[test]
fn piano_roll_coalesced_velocity_updates_skip_redundant_snapshots() {
    let mut state = PianoRollState::default();

    update(
        &mut state,
        AppMessage::Roll(PianoRollMessage::SetVelocities {
            velocities: vec![(2, 0.2)],
        }),
    );
    let undo_len = state.history.undo_len();
    update(
        &mut state,
        AppMessage::Roll(PianoRollMessage::SetVelocities {
            velocities: vec![(2, 0.4)],
        }),
    );

    assert_eq!(state.history.undo_len(), undo_len);
    assert!(
        state
            .notes
            .iter()
            .any(|note| { note.id == 2 && (note.velocity - 0.4).abs() < f32::EPSILON })
    );
}
