use super::*;

#[test]
fn piano_roll_velocity_drag_edits_selected_notes_together() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
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
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let handle = widget.velocity_handle_rect(lane, note);
    let start = Point::new(handle.center().x, lane.min.y + 4.0);
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

    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
    assert!(move_output.is_none());
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
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 240)
            )
        }),
        "velocity drag should paint the edited bars locally before release"
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
        PianoRollMessage::SetVelocity { ids, velocity } => {
            assert_eq!(ids, vec![2, 3]);
            assert!(
                (velocity - 0.28).abs() < 0.02,
                "dragging lower in the velocity lane should reduce the linked selected notes"
            );
            state.apply_roll_message(PianoRollMessage::SetVelocity { ids, velocity });
        }
        other => panic!("expected velocity edit message, got {other:?}"),
    }

    assert!(
        state
            .notes
            .iter()
            .filter(|note| [2, 3].contains(&note.id))
            .all(|note| (note.velocity - 0.28).abs() < 0.02)
    );
}
