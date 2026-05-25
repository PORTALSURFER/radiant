use super::*;

#[test]
fn piano_roll_drag_paints_new_note_length_before_commit() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 6.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 7.60), start.y);

    let press_output = widget.handle_input(
        bounds,
        WidgetInput::PointerDoubleClick {
            position: start,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert_eq!(
        press_output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::SetCursor { beat: 6.0 })
    );
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
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120))),
        "new-note paint drag should show a local note preview"
    );

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::CreateNote {
            pitch: 58,
            start_beat: 6.0,
            length_beats: 1.5,
        })
    );
}

#[test]
fn piano_roll_single_click_places_edit_cursor_without_creating_note() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let position = Point::new(
        x_for_beat_view(grid, state.viewport, 6.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
    );

    let press = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(press, Some(PianoRollMessage::SetCursor { beat: 6.0 }));
    assert_eq!(
        release.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::SetCursor { beat: 6.0 }),
        "single click release should settle the edit cursor rather than create a note"
    );
}

#[test]
fn piano_roll_snap_on_hover_cursor_uses_nearest_snap_point() {
    let state = PianoRollState::default();
    assert!(state.snap_enabled);
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let hover = Point::new(
        x_for_beat_view(grid, state.viewport, 6.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
    );

    widget.handle_input(bounds, WidgetInput::PointerMove { position: hover });
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let snapped_x = x_for_beat_view(grid, state.viewport, 6.0);
    assert!(overlay.iter().any(|primitive| matches!(
        primitive,
        PaintPrimitive::FillRect(fill)
            if (fill.rect.min.x - snapped_x).abs() < 0.01
                && fill.color == paint::translucent(ThemeTokens::default().text_muted, 90)
    )));
    assert!(
        !overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if (fill.rect.min.x - hover.x).abs() < 0.01
                    && fill.color == paint::translucent(ThemeTokens::default().text_muted, 90)
        )),
        "snap-on hover line should not paint at the raw pointer x"
    );
}

#[test]
fn piano_roll_snap_off_places_cursor_at_exact_pointer_beat() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleSnap);
    assert!(!state.snap_enabled);
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let position = Point::new(
        x_for_beat_view(grid, state.viewport, 6.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
    );

    let press = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());

    assert_eq!(press, Some(PianoRollMessage::SetCursor { beat: 6.10 }));
    assert!((widget.edit_cursor_x(grid).unwrap() - position.x).abs() < 0.01);
}

#[test]
fn piano_roll_single_drag_selects_time_range_with_overlay() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 4.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
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
                if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 42)
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
    let selected_widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
                if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 42)
        )),
        "committed time selection should remain visible after release"
    );
}

#[test]
fn piano_roll_synchronize_preserves_committed_time_selection_from_state() {
    let previous_state = PianoRollState::default();
    let previous = PianoRollWidget::new(
        previous_state.notes,
        previous_state.selected_note,
        previous_state.selected_notes,
        previous_state.selected_pitch,
        previous_state.edit_cursor_beat,
        previous_state.time_selection,
        previous_state.snap_enabled,
        previous_state.playhead_beat,
        previous_state.viewport,
        previous_state.tool,
    );
    let mut selected_state = PianoRollState::default();
    selected_state.apply_roll_message(PianoRollMessage::SetTimeSelection {
        start_beat: 4.0,
        end_beat: 7.5,
    });
    let mut next = PianoRollWidget::new(
        selected_state.notes,
        selected_state.selected_note,
        selected_state.selected_notes,
        selected_state.selected_pitch,
        selected_state.edit_cursor_beat,
        selected_state.time_selection,
        selected_state.snap_enabled,
        selected_state.playhead_beat,
        selected_state.viewport,
        selected_state.tool,
    );

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
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 5.0),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
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
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 5.0),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
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
            PaintPrimitive::FillRect(fill) if fill.color == paint::rgba(8, 12, 18, 255)
        )),
        "move preview should mask source notes"
    );
    assert!(
        move_overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.color == paint::translucent(ThemeTokens::default().grid_soft, 80)
        )),
        "source mask should redraw grid lines instead of leaving a black block"
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

#[test]
fn piano_roll_drag_routes_move_message() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
