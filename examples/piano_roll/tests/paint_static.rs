use super::*;

#[test]
fn piano_roll_widget_paints_keyboard_grid_notes_and_playhead() {
    let state = PianoRollState::default();
    let viewport = state.viewport;
    let widget = PianoRollWidget::new(
        state.notes,
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
    let mut primitives = Vec::new();
    let mut overlay = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count()
            > PITCH_ROWS
    );
    let keyboard = widget.keyboard_rect(bounds);
    assert!(
        row_height_for(keyboard, viewport) < 19.0,
        "default piano-roll rows are too short for 12px pitch labels"
    );
    assert!(
        keyboard_pitch_labels(&primitives, keyboard).is_empty(),
        "default keyboard pitch rows should hide labels when text cannot fit without clipping"
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
        "playhead should paint as a lightweight runtime overlay"
    );
}

#[test]
fn piano_roll_paints_all_keyboard_pitch_labels_when_rows_are_tall_enough() {
    let mut state = PianoRollState::default();
    state.viewport.visible_pitches = 8;
    let viewport = state.viewport;
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
    let keyboard = widget.keyboard_rect(bounds);
    assert!(
        row_height_for(keyboard, viewport) >= 19.0,
        "test viewport should make rows tall enough for labels"
    );

    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let pitch_labels = keyboard_pitch_labels(&primitives, keyboard);
    assert_eq!(
        pitch_labels.len(),
        viewport.row_count(),
        "every visible keyboard pitch row should get a label when labels fit"
    );
    assert!(
        pitch_labels.iter().any(|text| text.text.as_str() == "F#3"),
        "black-key pitch rows should be labeled too"
    );
    assert!(
        pitch_labels
            .iter()
            .all(|text| text.align == PaintTextAlign::Left),
        "keyboard pitch labels should be left aligned"
    );
    assert!(
        pitch_labels
            .iter()
            .all(|text| text.rect.min.y >= keyboard.min.y && text.rect.max.y <= keyboard.max.y),
        "keyboard pitch label text boxes should stay inside the keyboard paint bounds"
    );
}

#[test]
fn piano_roll_hides_keyboard_pitch_labels_when_rows_are_too_short() {
    let state = PianoRollState::default();
    let viewport = state.viewport;
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
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 300.0));
    let keyboard = widget.keyboard_rect(bounds);
    assert!(
        row_height_for(keyboard, viewport) < 19.0,
        "test bounds should force pitch rows below the label visibility threshold"
    );

    let mut primitives = Vec::new();
    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        !primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::Text(text)
                if text.rect.min.x >= keyboard.min.x && text.rect.max.x <= keyboard.max.x
        )),
        "keyboard pitch labels should be hidden once rows are too short to fit text"
    );
}

fn keyboard_pitch_labels(primitives: &[PaintPrimitive], keyboard: Rect) -> Vec<&PaintTextRun> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str().len() >= 2 => Some(text),
            _ => None,
        })
        .filter(|text| text.rect.min.x >= keyboard.min.x && text.rect.max.x <= keyboard.max.x)
        .collect::<Vec<_>>()
}

#[test]
fn piano_roll_viewport_scales_note_geometry() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ZoomTime { factor: 0.5 });
    state.apply_roll_message(PianoRollMessage::ZoomPitch { rows_delta: -8 });
    let widget = PianoRollWidget::new(
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
    let note = widget.note_by_id(2).expect("default note should exist");
    let zoomed = widget.note_rect(grid, note);
    let default_widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.edit_cursor_beat,
        state.time_selection,
        state.snap_enabled,
        state.playhead_beat,
        Default::default(),
        state.tool,
    );
    let unzoomed = default_widget.note_rect(grid, note);

    assert!(
        zoomed.width() > unzoomed.width(),
        "horizontal zoom should increase note width in screen space"
    );
    assert!(
        zoomed.height() > unzoomed.height(),
        "vertical zoom should increase row height in screen space"
    );
}

#[test]
fn piano_roll_note_geometry_can_move_past_vertical_viewport_edges_for_clipping() {
    let mut state = PianoRollState::default();
    state.viewport.pitch_start = 60;
    state.viewport.visible_pitches = 8;
    state.notes = vec![PianoNote {
        id: 101,
        pitch: 72,
        start_beat: 1.0,
        length_beats: 1.0,
        velocity: 0.7,
    }];
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

    assert!(
        rect.max.y < grid.min.y,
        "notes above the visible pitch range should project past the editor edge and be clipped by the paint clip"
    );
}

#[test]
fn piano_roll_clips_notes_to_editor_grid_with_radiant_clip() {
    let mut state = PianoRollState::default();
    state.notes = vec![
        PianoNote {
            id: 101,
            pitch: 55,
            start_beat: 2.0,
            length_beats: 2.0,
            velocity: 1.0,
        },
        PianoNote {
            id: 102,
            pitch: 57,
            start_beat: 6.0,
            length_beats: 2.0,
            velocity: 1.0,
        },
    ];
    state.selected_note = None;
    state.selected_notes.clear();
    state.viewport.beat_start = 3.0;
    state.viewport.visible_beats = 4.0;
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
    let grid = widget.editor_rect(bounds);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let clip_start = primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.rect == grid),
        )
        .expect("piano-roll notes should enter a Radiant clip for the editor grid");
    let clip_end = primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == widget.common.id),
        )
        .expect("piano-roll notes should leave the editor-grid clip");
    let note_rects = widget
        .notes
        .iter()
        .map(|note| widget.note_rect(grid, *note))
        .collect::<Vec<_>>();
    let note_fill_positions = note_rects
        .iter()
        .map(|rect| {
            primitives
                .iter()
                .position(
                    |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.rect == *rect),
                )
                .expect("raw note fill should be emitted inside the clip")
        })
        .collect::<Vec<_>>();

    assert!(clip_start < clip_end);
    assert!(
        note_fill_positions
            .iter()
            .all(|position| clip_start < *position && *position < clip_end),
        "note geometry should be clipped by Radiant clip primitives rather than per-rect clamping"
    );
    assert!(
        note_rects
            .iter()
            .any(|rect| rect.min.x < grid.min.x && rect.max.x > grid.min.x),
        "test should include a note that overhangs the left edge before renderer clipping"
    );
    assert!(
        note_rects
            .iter()
            .any(|rect| rect.min.x < grid.max.x && rect.max.x > grid.max.x),
        "test should include a note that overhangs the right edge before renderer clipping"
    );
}
