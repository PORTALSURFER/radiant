use super::*;

#[test]
fn piano_roll_drag_routes_move_message() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let start = widget.note_rect(grid, note).center();

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(
                start.x + grid.width() / TOTAL_BEATS,
                start.y - row_height_for(grid, state.viewport),
            ),
        },
    );

    assert!(output.is_none());
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
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120))),
        "moving a held note should paint a local drag preview"
    );
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: Point::new(
                start.x + grid.width() / TOTAL_BEATS,
                start.y - row_height_for(grid, state.viewport),
            ),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    assert!(matches!(
        release.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::MoveNote {
            id: 2,
            pitch: 56,
            ..
        })
    ));
}

#[test]
fn piano_roll_dragging_selected_note_moves_the_selected_group() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let start = widget.note_rect(grid, note).center();
    let end = Point::new(
        start.x + grid.width() / TOTAL_BEATS,
        start.y - row_height_for(grid, state.viewport),
    );

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(
        press.is_none(),
        "pressing an already selected note should keep the group selection"
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
        overlay
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120)))
            .count()
            >= 2,
        "drag preview should show every selected note moving together"
    );

    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("group drag release should commit a move");

    match release {
        PianoRollMessage::MoveNotes {
            ids,
            pitch_delta,
            beat_delta,
        } => {
            assert_eq!(ids, vec![2, 3]);
            assert_eq!(pitch_delta, 1);
            assert!((beat_delta - 1.0).abs() < f32::EPSILON);
            state.apply_roll_message(PianoRollMessage::MoveNotes {
                ids,
                pitch_delta,
                beat_delta,
            });
        }
        other => panic!("expected group move message, got {other:?}"),
    }

    assert_eq!(
        state.notes.iter().find(|note| note.id == 2).unwrap().pitch,
        56
    );
    assert_eq!(
        state.notes.iter().find(|note| note.id == 3).unwrap().pitch,
        61
    );
    assert_eq!(state.selected_notes, vec![2, 3]);
}
