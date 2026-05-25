use super::*;

#[test]
fn piano_roll_hover_uses_paint_only_runtime_overlay() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: widget.note_rect(grid, note).center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_note, Some(2));
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
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "hovered note should paint as a lightweight runtime overlay"
    );
}

#[test]
fn piano_roll_hover_lights_entire_note_tail() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let note_rect = widget.note_rect(grid, note);

    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: note_rect.center(),
        },
    );

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
                    if fill.rect.min.x > note_rect.min.x
                        && (fill.rect.max.x - note_rect.max.x).abs() < f32::EPSILON
                        && fill.rect.min.y == note_rect.min.y
                        && fill.rect.max.y == note_rect.max.y
            )
        }),
        "hover should light the whole trailing body of the note"
    );
}

#[test]
fn piano_roll_hover_lights_left_keyboard_note_row() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let keyboard = widget.keyboard_rect(bounds);
    let pitch = 60;
    let row = widget.keyboard_pitch_rect(keyboard, pitch);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: row.center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_pitch, Some(pitch));
    assert!(widget.prefers_pointer_move_paint_only());
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
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 85)
                        && fill.rect.min.x == keyboard.min.x
                        && fill.rect.max.x == keyboard.max.x
                        && fill.rect.min.y == row.min.y
                        && fill.rect.max.y == row.max.y
            )
        }),
        "hovering the left keyboard should light the current piano key row"
    );
}
