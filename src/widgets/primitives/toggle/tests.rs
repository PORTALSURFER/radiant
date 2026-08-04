use crate::gui::{
    input::InputTimestamp,
    types::{Point, Vector2},
};

use super::*;
use crate::widgets::interaction::{
    InteractionProvenance, PointerButton, PointerModifiers, WidgetKey,
};

#[test]
fn toggle_keyboard_activation_flips_active_state() {
    let mut toggle = ToggleWidget::new(8, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    let _ = toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Enter)),
        Some(ToggleMessage::ValueChanged {
            checked: true,
            provenance: InteractionProvenance::Keyboard { timestamp: None },
        })
    );
    assert_eq!(toggle.common.state.active, toggle.state.checked);

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(10.0, 10.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
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
                timestamp: None,
            },
        ),
        Some(ToggleMessage::ValueChanged {
            checked: false,
            provenance: InteractionProvenance::Pointer {
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            },
        })
    );
    assert_eq!(toggle.common.state.active, toggle.state.checked);
}

#[test]
fn toggle_pointer_release_preserves_native_provenance_and_flips_once() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    let mut toggle = ToggleWidget::new(9, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: false,
    };
    let timestamp = InputTimestamp::capture();

    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::pointer_press_with_timestamp(
                Point::new(12.0, 12.0),
                PointerButton::Primary,
                PointerModifiers::default(),
                Some(InputTimestamp::capture()),
            ),
        ),
        None
    );
    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::pointer_release_with_timestamp(
                Point::new(14.0, 14.0),
                PointerButton::Primary,
                modifiers,
                Some(timestamp),
            ),
        ),
        Some(ToggleMessage::ValueChanged {
            checked: true,
            provenance: InteractionProvenance::Pointer {
                modifiers,
                timestamp: Some(timestamp),
                sequence_range: None,
            },
        })
    );
    assert_eq!(toggle.common.state.active, toggle.state.checked);
    assert_eq!(
        toggle.handle_input(
            bounds,
            WidgetInput::pointer_release_with_timestamp(
                Point::new(14.0, 14.0),
                PointerButton::Primary,
                modifiers,
                Some(timestamp),
            ),
        ),
        None
    );
    assert!(toggle.state.checked);
    assert_eq!(toggle.common.state.active, toggle.state.checked);
}

#[test]
fn toggle_keyboard_activation_preserves_native_timestamp() {
    let mut toggle = ToggleWidget::new(10, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    let timestamp = InputTimestamp::capture();

    assert_eq!(
        toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        toggle.handle_input(
            Rect::default(),
            WidgetInput::key_press_with_timestamp(WidgetKey::Space, Some(timestamp)),
        ),
        Some(ToggleMessage::ValueChanged {
            checked: true,
            provenance: InteractionProvenance::Keyboard {
                timestamp: Some(timestamp),
            },
        })
    );
    assert_eq!(toggle.common.state.active, toggle.state.checked);
}

#[test]
fn toggle_vetoes_disabled_unfocused_unsupported_and_non_primary_input() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    let mut disabled = ToggleWidget::new(
        11,
        "Disabled",
        WidgetSizing::fixed(Vector2::new(88.0, 28.0)),
    )
    .with_checked(true);
    disabled.common.state.disabled = true;
    disabled.common.state.pressed = true;
    disabled.state.armed = true;

    assert_eq!(
        disabled.handle_input(bounds, WidgetInput::key_press(WidgetKey::Space)),
        None
    );
    assert_eq!(
        disabled.handle_input(bounds, WidgetInput::primary_release(Point::new(12.0, 12.0))),
        None
    );
    assert!(disabled.state.checked);
    assert_eq!(disabled.common.state.active, disabled.state.checked);
    assert!(!disabled.common.state.pressed);
    assert!(!disabled.state.armed);

    let mut unfocused = ToggleWidget::new(
        12,
        "Unfocused",
        WidgetSizing::fixed(Vector2::new(88.0, 28.0)),
    );
    assert_eq!(
        unfocused.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Enter)),
        None
    );
    assert!(!unfocused.state.checked);
    assert_eq!(unfocused.common.state.active, unfocused.state.checked);

    let mut unsupported = ToggleWidget::new(
        16,
        "Unsupported",
        WidgetSizing::fixed(Vector2::new(88.0, 28.0)),
    );
    assert_eq!(
        unsupported.handle_input(Rect::default(), WidgetInput::FocusChanged(true)),
        None
    );
    assert_eq!(
        unsupported.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Tab)),
        None
    );
    assert!(!unsupported.state.checked);
    assert_eq!(unsupported.common.state.active, unsupported.state.checked);

    let mut non_primary =
        ToggleWidget::new(13, "Pointer", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
    assert_eq!(
        non_primary.handle_input(bounds, WidgetInput::primary_press(Point::new(12.0, 12.0))),
        None
    );
    assert_eq!(
        non_primary.handle_input(
            bounds,
            WidgetInput::pointer_release(
                Point::new(12.0, 12.0),
                PointerButton::Secondary,
                PointerModifiers::default(),
            ),
        ),
        None
    );
    assert!(non_primary.common.state.pressed);
    assert!(!non_primary.state.checked);
    assert_eq!(non_primary.common.state.active, non_primary.state.checked);
}

#[test]
fn toggle_vetoes_out_of_bounds_and_unarmed_releases_but_rearms_on_reentry() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    let inside = Point::new(12.0, 12.0);
    let outside = Point::new(120.0, 12.0);
    let mut toggle = ToggleWidget::new(14, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_press(outside)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_release(inside)),
        None
    );
    assert!(!toggle.state.checked);
    assert_eq!(toggle.common.state.active, toggle.state.checked);

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_press(inside)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::pointer_move(outside)),
        None
    );
    assert!(!toggle.state.armed);
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_release(inside)),
        None
    );
    assert!(!toggle.state.checked);
    assert_eq!(toggle.common.state.active, toggle.state.checked);

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_press(inside)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_release(outside)),
        None
    );
    assert!(!toggle.state.checked);
    assert_eq!(toggle.common.state.active, toggle.state.checked);

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_press(inside)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::pointer_move(outside)),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::pointer_move(inside)),
        None
    );
    assert!(toggle.state.armed);
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_release(inside)),
        Some(ToggleMessage::ValueChanged {
            checked: true,
            provenance: InteractionProvenance::Pointer {
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            },
        })
    );
    assert_eq!(toggle.common.state.active, toggle.state.checked);
}

#[test]
fn toggle_focus_loss_cancels_pointer_activation() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
    let mut toggle = ToggleWidget::new(15, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));

    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_press(Point::new(12.0, 12.0))),
        None
    );
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::FocusChanged(false)),
        None
    );
    assert!(!toggle.common.state.focused);
    assert!(!toggle.common.state.pressed);
    assert!(!toggle.state.armed);
    assert_eq!(
        toggle.handle_input(bounds, WidgetInput::primary_release(Point::new(12.0, 12.0))),
        None
    );
    assert!(!toggle.state.checked);
    assert_eq!(toggle.common.state.active, toggle.state.checked);
}
