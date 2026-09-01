use crate::gui::types::{Point, Vector2};

use super::*;
use crate::widgets::interaction::{PointerButton, WidgetInput, WidgetKey};

#[test]
fn badge_releases_inside_bounds_emit_activation() {
    let mut badge = BadgeWidget::new(5, "Filter", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(72.0, 24.0));

    assert_eq!(
        badge.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(20.0, 30.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        None
    );
    assert!(badge.common.state.pressed);

    assert_eq!(
        badge.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(24.0, 32.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        ),
        Some(BadgeMessage::Activate)
    );
    assert!(!badge.common.state.pressed);
}

#[test]
fn focused_badge_enter_emits_activation() {
    let mut badge = BadgeWidget::new(6, "Active", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));

    let _ = badge.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

    assert_eq!(
        badge.handle_input(Rect::default(), WidgetInput::key_press(WidgetKey::Enter)),
        Some(BadgeMessage::Activate)
    );
}

#[test]
fn pointer_capture_cancellation_clears_badge_press_without_losing_focus() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(72.0, 24.0));
    let mut badge = BadgeWidget::new(8, "Filter", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));

    assert!(
        badge
            .handle_input(bounds, WidgetInput::primary_press(Point::new(20.0, 12.0)))
            .is_none()
    );
    assert!(badge.common.state.focused);
    assert!(badge.common.state.pressed);
    assert!(badge.state.armed);

    assert!(Widget::handle_pointer_capture_cancelled(&mut badge, bounds).is_none());
    assert!(badge.common.state.focused);
    assert!(!badge.common.state.pressed);
    assert!(!badge.state.armed);
    assert!(
        badge
            .handle_input(bounds, WidgetInput::primary_release(Point::new(20.0, 12.0)))
            .is_none()
    );
}

#[test]
fn badge_can_be_marked_active() {
    let badge = BadgeWidget::new(7, "Open", WidgetSizing::fixed(Vector2::new(72.0, 24.0)))
        .with_active(true);

    assert!(badge.common.state.active);
}
