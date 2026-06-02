use super::{ShortcutGesture, ShortcutLayer, ShortcutResolution, ShortcutStack};
use crate::gui::input::{KeyCode, KeyPress};

#[test]
fn shortcut_resolution_unhandled_has_no_action_or_chord() {
    let resolution = ShortcutResolution::<u8>::unhandled();

    assert_eq!(resolution.action, None);
    assert!(!resolution.handled);
    assert_eq!(resolution.pending_chord, None);
}

#[test]
fn shortcut_resolution_constructors_preserve_action_handled_and_chord_state() {
    let action = ShortcutResolution::action(7);
    assert_eq!(action.action, Some(7));
    assert!(action.handled);
    assert_eq!(action.pending_chord, None);

    let handled = ShortcutResolution::<u8>::handled();
    assert_eq!(handled.action, None);
    assert!(handled.handled);

    let chord = ShortcutResolution::<u8>::pending_chord(KeyPress::new(KeyCode::G));
    assert_eq!(chord.action, None);
    assert!(chord.handled);
    assert_eq!(chord.pending_chord, Some(KeyPress::new(KeyCode::G)));
}

#[test]
fn shortcut_gesture_matches_explicit_and_any_shift_modifiers() {
    assert!(ShortcutGesture::new(KeyCode::N).matches(KeyPress::new(KeyCode::N)));
    assert!(!ShortcutGesture::new(KeyCode::N).matches(KeyPress::with_shift(KeyCode::N)));
    assert!(ShortcutGesture::any_shift(KeyCode::N).matches(KeyPress::new(KeyCode::N)));
    assert!(ShortcutGesture::any_shift(KeyCode::N).matches(KeyPress::with_shift(KeyCode::N)));
    assert!(ShortcutGesture::with_command(KeyCode::A).matches(KeyPress::with_command(KeyCode::A)));
}

#[test]
fn shortcut_layer_resolves_actions_and_modal_misses() {
    let layer = ShortcutLayer::new()
        .bind(KeyPress::new(KeyCode::Escape), 1)
        .bind(ShortcutGesture::with_command(KeyCode::A), 2);

    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(1)
    );
    assert_eq!(
        layer.resolve(KeyPress::with_command(KeyCode::A)),
        ShortcutResolution::action(2)
    );
    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::unhandled()
    );

    let modal = ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), 3);
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(3)
    );
    assert_eq!(
        modal.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );
}

#[test]
fn shortcut_layer_bind_all_routes_equivalent_gestures_to_one_action() {
    let layer = ShortcutLayer::new().bind_all(
        [
            KeyPress::new(KeyCode::Delete),
            KeyPress::new(KeyCode::Backspace),
        ],
        7,
    );

    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::Delete)),
        ShortcutResolution::action(7)
    );
    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::Backspace)),
        ShortcutResolution::action(7)
    );
    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::unhandled()
    );
}

#[test]
fn shortcut_layer_modal_escape_dispatches_escape_and_consumes_misses() {
    let layer = ShortcutLayer::modal_escape(5);

    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(5)
    );
    assert_eq!(
        layer.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );
}

#[test]
fn shortcut_stack_resolves_layers_by_priority_and_uses_fallback_on_miss() {
    let stack = ShortcutStack::new()
        .push_when(
            false,
            ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), 99),
        )
        .push(ShortcutLayer::new().bind(KeyPress::new(KeyCode::Escape), 1))
        .push(ShortcutLayer::new().bind(KeyPress::new(KeyCode::N), 2));

    assert_eq!(stack.layers().len(), 2);
    assert_eq!(
        stack.resolve(KeyPress::new(KeyCode::Escape)),
        ShortcutResolution::action(1)
    );
    assert_eq!(
        stack.resolve_or_else(KeyPress::new(KeyCode::Tab), || ShortcutResolution::action(
            7
        )),
        ShortcutResolution::action(7)
    );
}

#[test]
fn shortcut_stack_stops_at_modal_layer_miss() {
    let stack = ShortcutStack::new()
        .push(ShortcutLayer::modal().bind(KeyPress::new(KeyCode::Escape), 1))
        .push(ShortcutLayer::new().bind(KeyPress::new(KeyCode::N), 2));

    assert_eq!(
        stack.resolve(KeyPress::new(KeyCode::N)),
        ShortcutResolution::handled()
    );
}
