use super::*;

#[test]
fn piano_roll_keyboard_press_lights_pitch_lane_and_selects_pitch() {
    let mut state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let keyboard = widget.keyboard_rect(bounds);
    let grid = widget.editor_rect(bounds);
    let pitch = 60;
    let row = widget.keyboard_pitch_rect(keyboard, pitch);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: row.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(widget.active_pitch, Some(pitch));
    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::SelectPitch(pitch))
    );
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let lane = widget.keyboard_pitch_rect(grid, pitch);
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == ThemeTokens::default().highlight_orange.with_alpha(72)
                        && fill.rect.min.y == lane.min.y
                        && fill.rect.max.y == lane.max.y
            )
        }),
        "pressing a piano key should immediately light the matching editor lane"
    );
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: row.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(release.is_none());
    assert_eq!(widget.active_pitch, None);
    assert_eq!(widget.hover_pitch, Some(pitch));

    state.apply_roll_message(PianoRollMessage::SelectPitch(pitch));
    assert_eq!(state.selected_pitch, Some(pitch));
    let selected_widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let mut primitives = Vec::new();
    selected_widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == ThemeTokens::default().highlight_blue.with_alpha(30)
                        && fill.rect.min.y == lane.min.y
                        && fill.rect.max.y == lane.max.y
            )
        }),
        "selected piano key should leave a persistent lane accent"
    );
}

#[test]
fn piano_roll_grid_hover_lights_matching_left_keyboard_note_row() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let keyboard = widget.keyboard_rect(bounds);
    let pitch = 57;
    let y =
        y_for_pitch_view(grid, state.viewport, pitch) + row_height_for(grid, state.viewport) * 0.5;

    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(grid.center().x, y),
        },
    );

    assert_eq!(widget.hover_pitch, Some(pitch));
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let row = widget.keyboard_pitch_rect(keyboard, pitch);
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == ThemeTokens::default().highlight_orange.with_alpha(85)
                        && fill.rect.min.y == row.min.y
                        && fill.rect.max.y == row.max.y
            )
        }),
        "hovering the grid should light the matching key on the left piano visual"
    );
}
