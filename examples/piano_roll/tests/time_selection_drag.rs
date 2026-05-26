use super::*;

#[test]
fn piano_roll_single_drag_selects_time_range_with_overlay() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 4.10),
        pitch_layout(grid, state.viewport).y_for_pitch(58) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 7.60), start.y);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(move_output.is_none());
    assert!(matches!(widget.drag, Some(PianoDrag::TimeSelection { .. })));
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
            PaintPrimitive::FillRect(fill)
                if fill.color == ThemeTokens::default().highlight_blue.with_alpha(42)
        )),
        "time-selection drag should paint a translucent beat-range overlay"
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
        .expect("time drag release should commit the selected time range");

    assert_eq!(
        release,
        PianoRollMessage::SetTimeSelection {
            start_beat: 4.0,
            end_beat: 7.5,
        }
    );
    let mut state = PianoRollState::default();
    state.apply_roll_message(release);
    assert!(
        state.selected_notes.is_empty(),
        "time selection should not select note nodes"
    );
    assert_eq!(state.selected_note, None);
    let selected_widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let mut persistent_overlay = Vec::new();
    selected_widget.append_runtime_overlay_paint(
        &mut persistent_overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        persistent_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == ThemeTokens::default().highlight_blue.with_alpha(42)
        )),
        "committed time selection should remain visible after release"
    );
}

#[test]
fn piano_roll_synchronize_preserves_committed_time_selection_from_state() {
    let previous_state = PianoRollState::default();
    let previous = PianoRollWidget::new(PianoRollWidgetParts::from_state(&previous_state));
    let mut selected_state = PianoRollState::default();
    selected_state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 7.5,
    });
    let mut next = PianoRollWidget::new(PianoRollWidgetParts::from_state(&selected_state));

    next.synchronize_from_previous(&previous);

    assert_eq!(
        next.time_selection,
        Some((4.0, 7.5)),
        "widget synchronization must not overwrite model-owned time selection state"
    );
}

#[test]
fn piano_roll_dragging_time_selection_moves_selection_as_object() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 7.5,
    });
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 5.0),
        pitch_layout(grid, state.viewport).y_for_pitch(58) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 6.0), start.y);

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
        Some(PianoDrag::MoveTimeSelection { .. })
    ));
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
        .expect("time selection drag should emit moved range");

    assert_eq!(
        release,
        PianoRollMessage::MoveTimeSelection {
            source_start_beat: 4.0,
            source_end_beat: 7.5,
            target_start_beat: 5.0,
        }
    );
}

#[test]
fn piano_roll_command_dragging_time_selection_copies_notes_to_target_range() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 7.5,
    });
    let original_note_count = state.notes.len();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 5.0),
        pitch_layout(grid, state.viewport).y_for_pitch(58) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 8.0), start.y);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    command: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("command-dragging time selection should emit copy action");

    match release {
        PianoRollMessage::CopyTimeSelection {
            source_start_beat,
            source_end_beat,
            target_start_beat,
        } => {
            assert_eq!(source_start_beat, 4.0);
            assert_eq!(source_end_beat, 7.5);
            assert_eq!(target_start_beat, 7.0);
            state.apply_roll_message(PianoRollMessage::CopyTimeSelection {
                source_start_beat,
                source_end_beat,
                target_start_beat,
            });
        }
        other => panic!("expected copy time selection message, got {other:?}"),
    }

    assert!(state.notes.len() >= original_note_count);
    assert_eq!(state.time_selection, Some((7.0, 10.5)));
    assert!(state.selected_notes.is_empty());
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 52 && (note.start_beat - 8.0).abs() < f32::EPSILON),
        "copied chunk should preserve relative note timing at the target"
    );
}
