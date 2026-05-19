use super::*;
use crate::gui::types::{Point, Vector2};
use crate::widgets::interaction::{PointerButton, WidgetKey};

#[test]
fn slider_pointer_drag_emits_clamped_values() {
    let mut slider = SliderWidget::new(9, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

    assert_eq!(
        slider.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(60.0, 14.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
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
}

#[test]
fn focused_slider_responds_to_keyboard_steps() {
    let mut slider = SliderWidget::new(10, 0.5, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));

    let _ = slider.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
    let Some(SliderMessage::ValueChanged { value }) = slider.handle_input(
        Rect::default(),
        WidgetInput::KeyPress(WidgetKey::ArrowRight),
    ) else {
        panic!("focused slider should emit an arrow-key change");
    };
    assert!((value - 0.55).abs() < f32::EPSILON);
    assert_eq!(
        slider.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Home)),
        Some(SliderMessage::ValueChanged { value: 0.0 })
    );
}
