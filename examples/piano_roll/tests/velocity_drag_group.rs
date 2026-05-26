use super::*;

#[test]
fn piano_roll_velocity_lane_marquee_selects_handles_for_group_velocity_drag() {
    let mut state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let first =
        widget.velocity_handle_rect(lane, widget.note_by_id(2).expect("note 2 should exist"));
    let second =
        widget.velocity_handle_rect(lane, widget.note_by_id(3).expect("note 3 should exist"));
    let start = Point::new(first.min.x - 4.0, first.min.y.min(second.min.y) - 4.0);
    let end = Point::new(second.max.x + 4.0, first.max.y.max(second.max.y) + 4.0);

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press.is_none());
    assert!(matches!(
        widget.drag,
        Some(PianoDrag::VelocityMarquee { .. })
    ));
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokeRect(stroke)
                if stroke.color == ThemeTokens::default().highlight_cyan.with_alpha(220)
                    && stroke.width == 2.0
        )),
        "velocity marquee should paint a lane-local selection rectangle"
    );

    let selection = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("velocity marquee release should select intersected handles");
    assert_eq!(
        selection,
        PianoRollMessage::SelectNotes {
            ids: vec![2, 3],
            mode: NoteSelectionMode::Replace,
        }
    );
    state.apply_roll_message(selection);
    assert_eq!(state.selected_notes, vec![2, 3]);

    let mut selected_widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let lane = selected_widget.velocity_rect(bounds);
    let selected_handle = selected_widget.velocity_handle_rect(
        lane,
        selected_widget.note_by_id(2).expect("note 2 should exist"),
    );
    selected_widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: selected_handle.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let drag_position = Point::new(
        selected_handle.center().x,
        lane.min.y + lane.height() * 0.25,
    );
    let move_output = selected_widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: drag_position,
        },
    );
    assert!(
        move_output
            .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
            .is_some_and(|message| matches!(message, PianoRollMessage::SetVelocities { .. })),
        "small group velocity drag should publish live values"
    );
    let group_velocity = selected_widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: drag_position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("releasing a marquee-selected handle should edit the selected group");

    match group_velocity {
        PianoRollMessage::SetVelocities { velocities } => {
            assert_eq!(
                velocities.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![2, 3]
            );
            assert!(
                (velocity_for(&velocities, 2) - 0.75).abs() < 0.02,
                "velocity drag should use the pointer y delta for the grabbed selected handle"
            );
        }
        other => panic!("expected grouped velocity edit, got {other:?}"),
    }
}

#[test]
fn piano_roll_group_velocity_drag_preserves_offsets_until_floor_or_ceiling() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("note 2 should exist");
    let handle = widget.velocity_handle_rect(lane, note);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: handle.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let lower_position = Point::new(handle.center().x, lane.min.y + lane.height() * 0.72);
    let live_lower = widget
        .handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: lower_position,
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("small relative velocity drag should emit live values");
    assert!(matches!(live_lower, PianoRollMessage::SetVelocities { .. }));
    let lower = widget
        .drag
        .as_ref()
        .and_then(PianoDrag::velocity_values)
        .expect("relative velocity drag should retain live preview values");

    assert!((velocity_for(&lower, 2) - 0.28).abs() < 0.02);
    assert!((velocity_for(&lower, 3) - 0.10).abs() < 0.02);

    let live_floor = widget
        .handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(handle.center().x, lane.max.y),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("small relative velocity drag should emit clamped live values");
    assert!(matches!(live_floor, PianoRollMessage::SetVelocities { .. }));
    let floor = widget
        .drag
        .as_ref()
        .and_then(PianoDrag::velocity_values)
        .expect("dragging to the floor should retain clamped preview values");

    assert_eq!(velocity_for(&floor, 2), 0.0);
    assert_eq!(velocity_for(&floor, 3), 0.0);
}

#[test]
fn piano_roll_stress_velocity_drag_updates_all_selected_notes_without_reselecting_each_move() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);
    let all_ids = state.notes.iter().map(|note| note.id).collect::<Vec<_>>();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: all_ids,
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(1).expect("stress note 1 should exist");
    let handle = widget.velocity_handle_rect(lane, note);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: handle.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let drag_position = Point::new(handle.center().x, lane.min.y);
    let move_output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: drag_position,
        },
    );
    assert!(
        move_output.is_none(),
        "stress velocity drag should not emit grouped updates on pointer moves"
    );
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: drag_position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("stress velocity drag should commit one grouped update on release");

    match output {
        PianoRollMessage::SetVelocities { velocities } => {
            assert_eq!(velocities.len(), STRESS_NOTE_COUNT);
            state.apply_roll_message(PianoRollMessage::SetVelocities { velocities });
        }
        other => panic!("expected stress velocity values, got {other:?}"),
    }
    assert_eq!(state.selected_notes.len(), STRESS_NOTE_COUNT);
    assert!(state.notes.iter().all(|note| note.velocity <= 1.0));
}
