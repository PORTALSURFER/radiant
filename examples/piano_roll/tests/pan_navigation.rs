use super::*;

#[test]
fn piano_roll_middle_mouse_drag_pans_view() {
    let mut state = PianoRollState::default();
    state.viewport.beat_start = 4.0;
    state.viewport.visible_beats = 8.0;
    state.viewport.pitch_start = 52;
    state.viewport.visible_pitches = 8;
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = grid.center();
    let end = Point::new(
        start.x - grid.width() * 0.125,
        start.y + row_height_for(grid, state.viewport) * 2.0,
    );

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(press.is_none());
    assert!(matches!(widget.drag, Some(PianoDrag::Pan { .. })));

    let output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta
        }) if beat_delta > 0.0 && pitch_delta == 2
    ));
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(release.is_none());
    assert!(widget.drag.is_none());
}

#[test]
fn piano_roll_middle_mouse_vertical_pan_accumulates_sub_row_motion() {
    let mut state = PianoRollState::default();
    state.viewport.visible_pitches = 8;
    state.viewport.pitch_start = 52;
    let mut widget = PianoRollWidget::new(PianoRollWidgetParts::from_state(&state));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let row_height = row_height_for(grid, state.viewport);
    let start = grid.center();

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    let first = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(start.x, start.y + row_height * 0.4),
        },
    );
    let second = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(start.x, start.y + row_height * 0.8),
        },
    );

    assert!(
        first.is_none(),
        "sub-row pan movement should wait until the accumulated drag reaches a row"
    );
    assert!(matches!(
        second.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta: 1
        }) if beat_delta.abs() < f32::EPSILON
    ));
}
