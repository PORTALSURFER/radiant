use super::*;
use crate::{
    gui::{
        input::InputTimestamp,
        types::{Point, Rect, Vector2},
    },
    widgets::interaction::{InteractionProvenance, PointerModifiers},
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
        &WidgetInput::pointer_move(Point::new(12.0, 22.0)),
        ActivationInputPolicy::pointer_only(),
    );
    assert!(state.hovered);

    handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::primary_press(Point::new(12.0, 22.0)),
        ActivationInputPolicy::pointer_only(),
    );
    assert!(state.pressed);
    assert!(!state.focused);

    let result = handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::primary_release(Point::new(12.0, 22.0)),
        ActivationInputPolicy::pointer_only(),
    );
    assert!(result.activated());
    assert_eq!(
        result.provenance(),
        Some(InteractionProvenance::Pointer {
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        })
    );
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
        &WidgetInput::primary_press(Point::new(12.0, 22.0)),
        ActivationInputPolicy::focusable(),
    );
    assert!(state.focused);

    let result = handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::key_press(WidgetKey::Space),
        ActivationInputPolicy::focusable(),
    );
    assert_eq!(
        result,
        ActivationInputResult::Activated {
            provenance: InteractionProvenance::Keyboard { timestamp: None },
        }
    );
}

#[test]
fn native_like_pointer_activation_preserves_accepted_release_provenance() {
    let mut state = WidgetState::default();
    let bounds = bounds();
    let modifiers = PointerModifiers {
        command: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let timestamp = InputTimestamp::capture();

    handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::primary_press(Point::new(12.0, 22.0)),
        ActivationInputPolicy::pointer_only(),
    );
    let result = handle_activation_input(
        &mut state,
        bounds,
        &WidgetInput::PointerRelease {
            position: Point::new(12.0, 22.0),
            button: PointerButton::Primary,
            modifiers,
            timestamp: Some(timestamp),
        },
        ActivationInputPolicy::pointer_only(),
    );

    assert_eq!(
        result.provenance(),
        Some(InteractionProvenance::Pointer {
            modifiers,
            timestamp: Some(timestamp),
            sequence_range: None,
        })
    );
}

#[test]
fn native_like_keyboard_activation_preserves_accepted_key_press_timestamp() {
    let mut state = WidgetState {
        focused: true,
        ..WidgetState::default()
    };
    let timestamp = InputTimestamp::capture();

    let result = handle_activation_input(
        &mut state,
        bounds(),
        &WidgetInput::KeyPress {
            key: WidgetKey::Enter,
            modifiers: crate::widgets::KeyboardModifiers::default(),
            repeat: false,
            timestamp: Some(timestamp),
        },
        ActivationInputPolicy::focusable(),
    );

    assert_eq!(
        result.provenance(),
        Some(InteractionProvenance::Keyboard {
            timestamp: Some(timestamp),
        })
    );
}

#[test]
fn activation_vetoes_return_none_and_preserve_precedence() {
    let bounds = bounds();
    let inside = Point::new(12.0, 22.0);
    let outside = Point::new(100.0, 22.0);

    let mut outside_release_state = WidgetState {
        pressed: true,
        ..WidgetState::default()
    };
    assert_eq!(
        handle_activation_input(
            &mut outside_release_state,
            bounds,
            &WidgetInput::primary_release(outside),
            ActivationInputPolicy::pointer_only(),
        ),
        ActivationInputResult::None
    );
    assert!(!outside_release_state.pressed);

    let mut non_primary_state = WidgetState {
        pressed: true,
        ..WidgetState::default()
    };
    assert_eq!(
        handle_activation_input(
            &mut non_primary_state,
            bounds,
            &WidgetInput::pointer_release(
                inside,
                PointerButton::Secondary,
                PointerModifiers::default(),
            ),
            ActivationInputPolicy::pointer_only(),
        ),
        ActivationInputResult::None
    );
    assert!(!non_primary_state.pressed);

    let mut unfocused_state = WidgetState::default();
    assert_eq!(
        handle_activation_input(
            &mut unfocused_state,
            bounds,
            &WidgetInput::key_press(WidgetKey::Enter),
            ActivationInputPolicy::focusable(),
        ),
        ActivationInputResult::None
    );

    let mut wrong_key_state = WidgetState {
        focused: true,
        ..WidgetState::default()
    };
    assert_eq!(
        handle_activation_input(
            &mut wrong_key_state,
            bounds,
            &WidgetInput::key_press(WidgetKey::Tab),
            ActivationInputPolicy::focusable(),
        ),
        ActivationInputResult::None
    );
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
        &WidgetInput::key_press(WidgetKey::Enter),
        ActivationInputPolicy::focusable(),
    );

    assert_eq!(result, ActivationInputResult::None);
    assert!(!state.pressed);
}
