use super::*;

#[test]
fn piano_roll_plain_vertical_wheel_zooms_pitch_only() {
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

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(0.0, -20.0),
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::ZoomViewport {
            time_factor: None,
            rows_delta: -2,
        })
    );

    let mut zoomed = PianoRollState::default();
    zoomed.apply_roll_message(PianoRollMessage::ZoomViewport {
        time_factor: None,
        rows_delta: -2,
    });

    assert_eq!(zoomed.viewport.visible_beats, TOTAL_BEATS);
    assert_eq!(zoomed.viewport.visible_pitches, 22);
}

#[test]
fn piano_roll_alt_vertical_wheel_zooms_time_only() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(0.0, -20.0),
            modifiers: PointerModifiers {
                alt: true,
                ..PointerModifiers::default()
            },
        },
    );

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::ZoomViewport {
            time_factor: Some(factor),
            rows_delta: 0
        }) if (factor - 0.8).abs() < f32::EPSILON
    ));

    let mut zoomed = PianoRollState::default();
    zoomed.apply_roll_message(PianoRollMessage::ZoomViewport {
        time_factor: Some(0.8),
        rows_delta: 0,
    });

    assert!((zoomed.viewport.visible_beats - 12.8).abs() < f32::EPSILON);
    assert_eq!(zoomed.viewport.visible_pitches, PITCH_ROWS);
}

#[test]
fn piano_roll_horizontal_wheel_still_pans_time_range() {
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

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(64.0, 0.0),
            modifiers: PointerModifiers::default(),
        },
    );

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            pitch_delta: 0,
            beat_delta
        }) if beat_delta > 0.0
    ));
}
