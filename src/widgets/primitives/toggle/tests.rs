use crate::gui::types::{Point, Vector2};

use super::*;
use crate::widgets::interaction::{PointerButton, WidgetKey};

#[test]
fn toggle_keyboard_activation_flips_active_state() {
    let mut toggle = ToggleWidget::new(8, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    let _ = toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
        Some(ToggleMessage::ValueChanged { checked: true })
    );
    assert!(toggle.common.state.active);

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        None
    );
    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        ),
        Some(ToggleMessage::ValueChanged { checked: false })
    );
}
