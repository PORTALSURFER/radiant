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
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
                    && fill.color == paint::translucent(ThemeTokens::default().highlight_cyan, 72)
        )),
        "velocity drag should not paint a note-hover overlay that disappears on release"
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
    let live_widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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

#[test]
fn piano_roll_velocity_drag_starts_only_from_handle_rect() {
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
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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

#[test]
fn piano_roll_velocity_lane_marquee_selects_handles_for_group_velocity_drag() {
    let mut state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
                if stroke.color == paint::translucent(ThemeTokens::default().highlight_cyan, 220)
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

    let mut selected_widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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

fn velocity_for(velocities: &[(u32, f32)], id: u32) -> f32 {
    velocities
        .iter()
        .find_map(|(note_id, velocity)| (*note_id == id).then_some(*velocity))
        .unwrap_or_else(|| panic!("missing velocity for note {id}"))
}
