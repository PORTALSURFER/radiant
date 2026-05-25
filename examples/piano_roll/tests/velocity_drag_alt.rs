use super::*;

#[test]
fn piano_roll_alt_dragging_note_adjusts_velocity() {
    let mut state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let start = widget.note_rect(grid, note).center();
    let raised = Point::new(start.x, start.y - 32.0);

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers {
                alt: true,
                ..PointerModifiers::default()
            },
        },
    );

    assert!(
        press.is_none(),
        "selected alt-drag starts without reselection"
    );
    assert!(matches!(
        widget.drag,
        Some(PianoDrag::VelocityRelative { ref ids, .. }) if ids == &[2]
    ));

    let live = widget
        .handle_input(bounds, WidgetInput::PointerMove { position: raised })
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("alt note velocity drag should publish live values for small selections");
    match live {
        PianoRollMessage::SetVelocities { velocities } => {
            assert!(
                (velocity_for(&velocities, 2) - 1.0).abs() < f32::EPSILON,
                "dragging upward should raise the note velocity"
            );
            state.apply_roll_message(PianoRollMessage::SetVelocities { velocities });
        }
        other => panic!("expected live velocity values, got {other:?}"),
    }

    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: raised,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    alt: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("alt note velocity drag should commit on release");
    assert!(matches!(release, PianoRollMessage::SetVelocities { .. }));
}

#[test]
fn piano_roll_alt_dragging_unselected_note_selects_it_before_velocity_drag() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(4).expect("unselected note should exist");
    let start = widget.note_rect(grid, note).center();

    let press = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: start,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    alt: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("alt-dragging an unselected note should select it immediately");

    assert_eq!(press, PianoRollMessage::SelectNote(4));
    assert_eq!(widget.selected_notes, vec![4]);
    assert!(matches!(
        widget.drag,
        Some(PianoDrag::VelocityRelative { ref ids, .. }) if ids == &[4]
    ));
}

#[test]
fn piano_roll_velocity_drag_starts_only_from_handle_rect() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let handle = widget.velocity_handle_rect(lane, note);
    let outside_handle = Point::new(handle.center().x, lane.max.y - 6.0);

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: outside_handle,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert!(press.is_none());
    assert!(
        matches!(widget.drag, Some(PianoDrag::VelocityMarquee { .. })),
        "pressing a selected velocity column outside the handle should not start value drag"
    );
}

#[test]
fn piano_roll_velocity_drag_selects_unselected_handle_on_press_without_modifiers() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(4).expect("unselected note should exist");
    let handle = widget.velocity_handle_rect(lane, note);

    let press = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: handle.center(),
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("pressing an unselected velocity handle should select it immediately");

    assert_eq!(press, PianoRollMessage::SelectNote(4));
    assert_eq!(widget.selected_notes, vec![4]);
    assert!(matches!(
        widget.drag,
        Some(PianoDrag::Velocity { ref ids, .. }) if ids == &[4]
    ));
}
