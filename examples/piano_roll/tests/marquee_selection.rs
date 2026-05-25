use super::*;

#[test]
fn piano_roll_marquee_preview_lights_intersecting_notes_like_hover() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        PianoRollTool::Select,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let note_rect = widget.note_rect(grid, note);
    let start = Point::new(note_rect.min.x - 6.0, note_rect.min.y - 6.0);
    let end = Point::new(note_rect.max.x + 6.0, note_rect.max.y + 6.0);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.color == ThemeTokens::default().highlight_orange
                        && stroke.width == 2.0
                        && stroke.rect == note_rect
            )
        }),
        "notes intersecting the active marquee should use the orange hover-style highlight"
    );
}

#[test]
fn piano_roll_shift_drag_uses_marquee_selection_in_paint_tool() {
    let state = PianoRollState::default();
    assert_eq!(state.tool, PianoRollTool::Paint);
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(grid.min.x + 1.0, grid.min.y + 1.0);
    let end = Point::new(grid.min.x + grid.width() * 0.33, grid.max.y - 1.0);
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press.is_none());
    assert!(move_output.is_none());
    assert!(matches!(widget.drag, Some(PianoDrag::Marquee { .. })));
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("shift marquee release should emit selection");

    assert!(matches!(
        release,
        PianoRollMessage::SelectNotes {
            mode: NoteSelectionMode::Replace,
            ..
        }
    ));
}

#[test]
fn piano_roll_shift_command_marquee_adds_to_existing_selection() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(3).expect("target note should exist");
    let note_rect = widget.note_rect(grid, note);
    let start = Point::new(note_rect.min.x - 2.0, note_rect.min.y - 2.0);
    let end = Point::new(note_rect.max.x + 2.0, note_rect.max.y + 2.0);
    let modifiers = PointerModifiers {
        shift: true,
        command: true,
        ..PointerModifiers::default()
    };

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("shift+command marquee release should emit selection");

    match release {
        PianoRollMessage::SelectNotes { ids, mode } => {
            assert_eq!(mode, NoteSelectionMode::Add);
            assert_eq!(ids, vec![3]);
            state.apply_roll_message(PianoRollMessage::SelectNotes { ids, mode });
        }
        other => panic!("expected additive marquee selection, got {other:?}"),
    }

    assert_eq!(state.selected_notes, vec![2, 3]);
}
