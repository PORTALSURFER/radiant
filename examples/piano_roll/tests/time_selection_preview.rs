use super::*;

#[test]
fn piano_roll_moving_time_selection_slices_source_and_overwrites_target() {
    let mut state = PianoRollState::default();
    state.selected_note = Some(101);
    state.selected_notes = vec![101];
    state.notes = vec![
        PianoNote {
            id: 101,
            pitch: 60,
            start_beat: 3.0,
            length_beats: 3.0,
            velocity: 0.8,
        },
        PianoNote {
            id: 102,
            pitch: 62,
            start_beat: 4.5,
            length_beats: 0.5,
            velocity: 0.7,
        },
        PianoNote {
            id: 103,
            pitch: 64,
            start_beat: 8.25,
            length_beats: 0.5,
            velocity: 0.6,
        },
    ];

    state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 5.0,
    });
    state.apply_roll_message(PianoRollMessage::MoveTimeSelection {
        source_start_beat: 4.0,
        source_end_beat: 5.0,
        target_start_beat: 8.0,
    });

    assert!(note_exists(&state.notes, 60, 3.0, 1.0));
    assert!(note_exists(&state.notes, 60, 5.0, 1.0));
    assert!(note_exists(&state.notes, 60, 8.0, 1.0));
    assert!(note_exists(&state.notes, 62, 8.5, 0.5));
    assert!(
        !state
            .notes
            .iter()
            .any(|note| note.pitch == 64 && note.start_beat >= 8.0 && note.end_beat() <= 9.0),
        "drop target notes inside the moved slice range should be overwritten"
    );
    assert_eq!(state.time_selection, Some((8.0, 9.0)));
    assert!(state.selected_notes.is_empty());
    assert_eq!(state.selected_note, None);
}

#[test]
fn piano_roll_moving_time_selection_previews_clipped_notes_at_target() {
    let mut state = PianoRollState::default();
    state.notes = vec![PianoNote {
        id: 101,
        pitch: 60,
        start_beat: 3.0,
        length_beats: 3.0,
        velocity: 0.8,
    }];
    state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 5.0,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 4.25),
        y_for_pitch_view(grid, state.viewport, 60) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 8.25), start.y);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    let preview = widget.time_slice_preview_notes(grid);
    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].pitch, 60);
    assert!((preview[0].start_beat - 8.0).abs() < f32::EPSILON);
    assert!((preview[0].length_beats - 1.0).abs() < f32::EPSILON);
}

#[test]
fn piano_roll_time_selection_move_preview_keeps_grid_and_command_restores_source() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 5.0,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 4.25),
        y_for_pitch_view(grid, state.viewport, 60) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 8.25), start.y);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    let mut move_overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut move_overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        move_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == paint::rgba(8, 12, 18, 255)
                    && fill.rect.width() < grid.width()
        )),
        "move preview should mask source notes"
    );
    assert!(
        !move_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == paint::rgba(8, 12, 18, 255) && fill.rect == grid
        )),
        "move preview must not repaint the full grid background over the base note layer"
    );
    assert!(
        move_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == paint::translucent(ThemeTokens::default().grid_soft, 80)
        )),
        "source mask should redraw grid lines instead of leaving a black block"
    );
    assert!(
        !move_overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::Text(_))),
        "runtime overlay should not repaint beat labels over the top ruler"
    );

    widget.handle_input(
        bounds,
        WidgetInput::PointerModifiersChanged {
            modifiers: PointerModifiers {
                command: true,
                ..PointerModifiers::default()
            },
        },
    );
    let mut copy_overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut copy_overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        !copy_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill) if fill.color == paint::rgba(8, 12, 18, 255)
        )),
        "copy preview should leave the source notes and grid untouched"
    );
}

fn note_exists(notes: &[PianoNote], pitch: i32, start_beat: f32, length_beats: f32) -> bool {
    notes.iter().any(|note| {
        note.pitch == pitch
            && (note.start_beat - start_beat).abs() < f32::EPSILON
            && (note.length_beats - length_beats).abs() < f32::EPSILON
    })
}
