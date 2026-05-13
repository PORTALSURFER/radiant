use super::*;

#[test]
fn toggle_updates_active_state_when_activated() {
    let mut toggle = ToggleWidget::new(2, "Enabled", WidgetSizing::fixed(Vector2::new(84.0, 28.0)));

    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Space)),
        Some(ToggleMessage::ValueChanged { checked: true })
    );
    assert!(toggle.common.state.active);
}

#[test]
fn text_input_edits_and_submits_single_line_values() {
    let mut input = TextInputWidget::new(
        3,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    );

    let _ = input.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
    input.state.caret = 1;
    input.state.selection_anchor = 1;

    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::Character('z')),
        Some(TextInputMessage::Changed {
            value: String::from("azb"),
        })
    );
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Backspace)),
        Some(TextInputMessage::Changed {
            value: String::from("ab"),
        })
    );
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(TextInputMessage::Submitted {
            value: String::from("ab"),
        })
    );
}

#[test]
fn text_input_accepts_text_input_only_while_focused_and_editable() {
    let mut input = TextInputWidget::new(
        3,
        "ab",
        WidgetSizing::new(Vector2::new(96.0, 28.0), Vector2::new(160.0, 28.0)),
    );

    assert!(!input.accepts_text_input());

    let _ = input.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
    assert!(input.accepts_text_input());

    input.common.state.read_only = true;
    assert!(!input.accepts_text_input());
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::Character('z')),
        None
    );
    assert_eq!(input.state.value, "ab");

    input.common.state.read_only = false;
    input.common.state.disabled = true;
    assert!(!input.accepts_text_input());
    assert_eq!(
        input.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Backspace)),
        None
    );
    assert_eq!(input.state.value, "ab");
}

#[test]
fn scrollbar_drag_and_track_click_emit_normalized_offsets() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 120.0));
    let mut scrollbar = ScrollbarWidget::new(
        4,
        ScrollbarAxis::Vertical,
        WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
    );
    scrollbar.props.viewport_fraction = 0.25;
    let thumb = scrollbar.thumb_rect(bounds);
    let grip_y = thumb.min.y + thumb.height() * 0.5;

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, grip_y),
                button: PointerButton::Primary,
            },
        ),
        None
    );
    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(6.0, 96.0),
            },
        ),
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 0.9,
        })
    );
    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(6.0, 96.0),
                button: PointerButton::Primary,
            },
        ),
        None
    );

    assert_eq!(
        scrollbar.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(6.0, 12.0),
                button: PointerButton::Primary,
            },
        ),
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: 0.0,
        })
    );
}

#[test]
fn slider_drag_and_keyboard_emit_normalized_values() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));
    let mut slider = SliderWidget::new(14, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));

    assert_eq!(
        slider.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(60.0, 14.0),
                button: PointerButton::Primary,
            },
        ),
        Some(SliderMessage::ValueChanged { value: 0.5 })
    );
    assert_eq!(
        slider.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(180.0, 14.0),
            },
        ),
        Some(SliderMessage::ValueChanged { value: 1.0 })
    );

    let _ = slider.handle_input(bounds, WidgetInput::FocusChanged(true));
    assert_eq!(
        slider.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Home)),
        Some(SliderMessage::ValueChanged { value: 0.0 })
    );
}
