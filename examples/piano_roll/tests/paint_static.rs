use super::*;

#[test]
fn piano_roll_widget_paints_keyboard_grid_notes_and_playhead() {
    let state = PianoRollState::default();
    let viewport = state.viewport;
    let widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
        row_height_for(keyboard, viewport) * 12.0 >= 19.0,
        "default piano-roll octave chunks should be tall enough for octave labels"
    );
    let keyboard_labels = keyboard_pitch_labels(&primitives, keyboard);
    assert_eq!(
        keyboard_labels
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>(),
        ["C3", "C4"],
        "compact keyboard should show octave C labels instead of per-note labels"
    );
    assert!(
        keyboard_labels
            .iter()
            .all(|text| text.align == PaintTextAlign::Left),
        "compact keyboard labels should stay left aligned"
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
    let widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
fn piano_roll_merges_keyboard_labels_into_octave_chunks_when_rows_are_too_short() {
    let state = PianoRollState::default();
    let viewport = state.viewport;
    let widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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

    let pitch_labels = keyboard_pitch_labels(&primitives, keyboard);
    assert_eq!(
        pitch_labels
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>(),
        ["C3", "C4"],
        "keyboard labels should collapse to visible octave roots when pitch rows are too short"
    );
    assert!(
        pitch_labels
            .iter()
            .all(|text| text.text.as_str().starts_with('C')),
        "compact keyboard should not paint non-octave pitch labels"
    );
    let octave_strokes = keyboard_strokes(&primitives, keyboard)
        .into_iter()
        .filter(|stroke| stroke.rect.height() > row_height_for(keyboard, viewport) * 6.0)
        .count();
    assert!(
        octave_strokes >= 2,
        "compact keyboard should draw octave-sized chunks instead of per-note separators"
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

fn keyboard_strokes(primitives: &[PaintPrimitive], keyboard: Rect) -> Vec<&PaintStrokeRect> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::StrokeRect(stroke) => Some(stroke),
            _ => None,
        })
        .filter(|stroke| stroke.rect.min.x >= keyboard.min.x && stroke.rect.max.x <= keyboard.max.x)
        .collect::<Vec<_>>()
}
