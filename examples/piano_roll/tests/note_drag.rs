use super::*;

#[test]
fn piano_roll_drag_paints_new_note_length_before_commit() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == ThemeTokens::default().highlight_blue.with_alpha(120))),
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
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
                && fill.color == ThemeTokens::default().text_muted.with_alpha(90)
    )));
    assert!(
        !overlay.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::FillRect(fill)
                if (fill.rect.min.x - hover.x).abs() < 0.01
                    && fill.color == ThemeTokens::default().text_muted.with_alpha(90)
        )),
        "snap-on hover line should not paint at the raw pointer x"
    );
}

#[test]
fn piano_roll_snap_off_places_cursor_at_exact_pointer_beat() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleSnap);
    assert!(!state.snap_enabled);
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
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
