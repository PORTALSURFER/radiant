use super::*;
use crate::{
    gui::types::{Point, Rect, Vector2},
    widgets::interaction::PointerModifiers,
};

fn bounds() -> Rect {
    Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(80.0, 24.0))
}

#[test]
fn pointer_activation_tracks_hover_press_and_release() {
    let mut state = WidgetState::default();
    let bounds = bounds();

    handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::PointerMove {
            position: Point::new(12.0, 22.0),
        },
        ActivationInputPolicy::pointer_only(),
    );
    assert!(state.hovered);

    handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::PointerPress {
            position: Point::new(12.0, 22.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
        ActivationInputPolicy::pointer_only(),
    );
    assert!(state.pressed);
    assert!(!state.focused);

    let result = handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::PointerRelease {
            position: Point::new(12.0, 22.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
        ActivationInputPolicy::pointer_only(),
    );
    assert!(result.activated());
    assert!(!state.pressed);
    assert!(state.hovered);
}

#[test]
fn focusable_activation_focuses_on_press_and_uses_keyboard() {
    let mut state = WidgetState::default();
    let bounds = bounds();

    handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::PointerPress {
            position: Point::new(12.0, 22.0),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
        ActivationInputPolicy::focusable(),
    );
    assert!(state.focused);

    let result = handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::KeyPress(WidgetKey::Space),
        ActivationInputPolicy::focusable(),
    );
    assert_eq!(result, ActivationInputResult::Activated);
}

#[test]
fn disabled_activation_clears_pressed_and_ignores_input() {
    let mut state = WidgetState {
        pressed: true,
        disabled: true,
        ..WidgetState::default()
    };

    let result = handle_activation_input(
        &mut state,
        bounds(),
        &WidgetInput::KeyPress(WidgetKey::Enter),
        ActivationInputPolicy::focusable(),
    );

    assert_eq!(result, ActivationInputResult::None);
    assert!(!state.pressed);
}
