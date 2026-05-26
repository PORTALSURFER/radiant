use super::*;

#[test]
fn piano_roll_velocity_drag_edits_selected_notes_together() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let handle = widget.velocity_handle_rect(lane, note);
    let start = handle.center();
    let end = Point::new(handle.center().x, lane.min.y + lane.height() * 0.72);

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
        "velocity drag should not commit a state update on press"
    );
    assert_eq!(
        widget.hover_note, None,
        "velocity drag should not keep an editor hover overlay on the edited note"
    );

    let move_message = widget
        .handle_input(bounds, WidgetInput::PointerMove { position: end })
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("small velocity drags should publish live values for note repaint");
    match move_message {
        PianoRollMessage::SetVelocities { velocities } => {
            assert_eq!(
                velocities.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![2, 3]
            );
            assert!(
                (velocity_for(&velocities, 2) - 0.28).abs() < 0.02,
                "dragging lower in the velocity lane should update selected notes before release"
            );
            state.apply_roll_message(PianoRollMessage::SetVelocities { velocities });
        }
        other => panic!("expected live velocity edit message, got {other:?}"),
    }
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let grid = widget.editor_rect(bounds);
    let edited_note_rect = widget.note_rect(grid, note);
    assert!(
        !overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect == edited_note_rect
                    && fill.color == ThemeTokens::default().highlight_cyan.with_alpha(72)
        )),
        "velocity drag should not paint a note-hover overlay that disappears on release"
    );
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == ThemeTokens::default().highlight_orange.with_alpha(240)
            )
        }),
        "velocity drag should paint the edited bars locally before release"
    );
    let live_widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let grid = live_widget.editor_rect(bounds);
    let live_note = state
        .notes
        .iter()
        .copied()
        .find(|note| note.id == 2)
        .expect("edited note should still exist");
    let mut live_paint = Vec::new();
    live_widget.append_paint(
        &mut live_paint,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let live_alpha = fill_alpha_for_rect(&live_paint, live_widget.note_rect(grid, live_note));
    assert!(
        live_alpha < 130,
        "small live velocity updates should lower the note fill alpha before release"
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
        .expect("velocity drag release should commit the edited value");

    match release {
        PianoRollMessage::SetVelocities { velocities } => {
            assert_eq!(
                velocities.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![2, 3]
            );
            assert!(
                (velocity_for(&velocities, 2) - 0.28).abs() < 0.02,
                "dragging lower in the velocity lane should reduce the linked selected notes"
            );
            state.apply_roll_message(PianoRollMessage::SetVelocities { velocities });
        }
        other => panic!("expected velocity edit message, got {other:?}"),
    }

    let note_2 = state
        .notes
        .iter()
        .find(|note| note.id == 2)
        .expect("note 2 should exist");
    let note_3 = state
        .notes
        .iter()
        .find(|note| note.id == 3)
        .expect("note 3 should exist");
    assert!((note_2.velocity - 0.28).abs() < 0.02);
    assert!((note_3.velocity - 0.10).abs() < 0.02);
}
