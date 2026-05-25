use super::*;

#[test]
fn piano_roll_velocity_lane_paints_dense_pillars_for_stress_notes() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);
    let widget = PianoRollWidget::new(
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
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Velocity")
    ));
    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count()
            > STRESS_NOTE_COUNT,
        "dense velocity lane should add stem and handle primitives for synthetic notes"
    );
}

#[test]
fn piano_roll_velocity_pillars_align_to_note_start() {
    let state = PianoRollState::default();
    let widget = PianoRollWidget::new(
        state.notes.clone(),
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
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let stem = widget.velocity_preview_stem_rect(lane, note);
    let expected_x = x_for_beat_view(lane, state.viewport, note.start_beat);

    assert!(
        (stem.center().x - expected_x).abs() < f32::EPSILON,
        "velocity pillar should line up with the start of the note"
    );
}

#[test]
fn piano_roll_velocity_handle_hover_paints_runtime_highlight() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
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
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let handle = widget.velocity_handle_rect(lane, note);

    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: handle.center(),
        },
    );
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert_eq!(widget.hover_velocity_note, Some(note.id));
    assert!(overlay.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::StrokeRect(stroke)
                if stroke.rect == handle
                    && stroke.color == paint::translucent(ThemeTokens::default().text_primary, 245)
                    && stroke.width == 2.0
        )
    }));
}

#[test]
fn piano_roll_selected_velocity_handle_uses_note_selection_orange() {
    let state = PianoRollState::default();
    let widget = PianoRollWidget::new(
        state.notes.clone(),
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
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let handle = widget.velocity_handle_rect(lane, note).clamp_to(lane);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.iter().any(|primitive| {
        matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if fill.rect == handle && fill.color == ThemeTokens::default().highlight_orange
        )
    }));
}

#[test]
fn piano_roll_note_fill_alpha_tracks_velocity_with_visible_floor() {
    let mut state = PianoRollState::default();
    state.notes = vec![
        PianoNote {
            id: 101,
            pitch: 55,
            start_beat: 1.0,
            length_beats: 1.0,
            velocity: 0.0,
        },
        PianoNote {
            id: 102,
            pitch: 57,
            start_beat: 3.0,
            length_beats: 1.0,
            velocity: 1.0,
        },
    ];
    state.selected_note = None;
    state.selected_notes.clear();
    let widget = PianoRollWidget::new(
        state.notes.clone(),
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
    let quiet_rect = widget.note_rect(grid, state.notes[0]);
    let loud_rect = widget.note_rect(grid, state.notes[1]);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let quiet_alpha = fill_alpha_for_rect(&primitives, quiet_rect);
    let loud_alpha = fill_alpha_for_rect(&primitives, loud_rect);

    assert_eq!(quiet_alpha, 51);
    assert_eq!(loud_alpha, 255);
    assert!(
        loud_alpha > quiet_alpha,
        "higher velocity notes should paint more opaque than low velocity notes"
    );
}

#[test]
fn piano_roll_selected_note_fill_alpha_tracks_velocity_while_border_marks_selection() {
    let mut state = PianoRollState::default();
    state.notes = vec![PianoNote {
        id: 101,
        pitch: 55,
        start_beat: 1.0,
        length_beats: 1.0,
        velocity: 0.0,
    }];
    state.selected_note = Some(101);
    state.selected_notes = vec![101];
    let widget = PianoRollWidget::new(
        state.notes.clone(),
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
    let rect = widget.note_rect(grid, state.notes[0]);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert_eq!(fill_alpha_for_rect(&primitives, rect), 51);
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.rect == rect
                        && stroke.color == ThemeTokens::default().highlight_orange
                        && stroke.width == 2.0
            )
        }),
        "selection should stay visible through the orange border even when low velocity dims the fill"
    );
}
