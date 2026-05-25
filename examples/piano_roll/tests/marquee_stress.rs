use super::*;

#[test]
fn piano_roll_stress_mode_generates_thousands_of_notes_for_marquee_selection() {
    let mut state = PianoRollState::default();

    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);

    assert_eq!(state.notes.len(), STRESS_NOTE_COUNT);
    assert_eq!(state.tool, PianoRollTool::Select);
    assert!(state.selected_notes.is_empty());
    assert!(
        state.status().contains("stress 4096 notes"),
        "status should make the dense GUI stress load visible"
    );
}

#[test]
fn piano_roll_marquee_selects_thousands_of_notes_with_paint_only_preview() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);
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
    let start = Point::new(grid.min.x + 1.0, grid.min.y + 1.0);
    let end = Point::new(grid.min.x + grid.width() * 0.66, grid.max.y - 1.0);

    let press_output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press_output.is_none());
    assert!(move_output.is_none());
    assert!(widget.prefers_pointer_move_paint_only());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "marquee drag should paint a local selection rectangle"
    );

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let message = output
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("marquee release should emit a selection message");

    match message {
        PianoRollMessage::SelectNotes { ids, mode } => {
            assert_eq!(mode, NoteSelectionMode::Replace);
            assert!(
                ids.len() > 2_000,
                "wide marquee should select thousands of dense synthetic notes"
            );
            state.apply_roll_message(PianoRollMessage::SelectNotes { ids, mode });
        }
        other => panic!("expected marquee selection message, got {other:?}"),
    }
    assert!(state.selected_notes.len() > 2_000);
}
