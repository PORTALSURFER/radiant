use super::*;

#[test]
fn piano_roll_modifier_click_adds_and_toggles_note_selection() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(3).expect("note should exist");
    let position = widget.note_rect(grid, note).center();

    let shift_output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    shift: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());
    assert_eq!(
        shift_output,
        Some(PianoRollMessage::SelectNotes {
            ids: vec![3],
            mode: NoteSelectionMode::Add,
        })
    );

    let command_output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    command: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());
    assert_eq!(
        command_output,
        Some(PianoRollMessage::SelectNotes {
            ids: vec![3],
            mode: NoteSelectionMode::Toggle,
        })
    );
    assert!(widget.drag.is_none());
}

#[test]
fn piano_roll_selected_notes_paint_persistent_orange_borders() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    for id in [2, 3] {
        let note = widget.note_by_id(id).expect("selected note should exist");
        let rect = widget.note_rect(grid, note);
        assert!(
            primitives.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::StrokeRect(stroke)
                        if stroke.color == ThemeTokens::default().highlight_orange
                            && stroke.width == 2.0
                            && stroke.rect == rect
                )
            }),
            "selected notes should keep a persistent orange border"
        );
    }
}

#[test]
fn piano_roll_created_note_cuts_existing_note_into_left_and_right_fragments() {
    let mut state = PianoRollState::default();
    state.notes = vec![PianoNote {
        id: 1,
        pitch: 60,
        start_beat: 1.0,
        length_beats: 4.0,
        velocity: 0.8,
    }];
    state.selected_note = Some(1);

    state.apply_roll_message(PianoRollMessage::CreateNote {
        pitch: 60,
        start_beat: 2.0,
        length_beats: 1.0,
    });

    assert_eq!(state.notes.len(), 3);
    let mut ids = state.notes.iter().map(|note| note.id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3);
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 1.0 && note.length_beats == 1.0)
    );
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 2.0 && note.length_beats == 1.0)
    );
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 3.0 && note.length_beats == 2.0)
    );
}
