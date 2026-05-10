//! Badge pointer and keyboard activation behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{BadgeMessage, PointerButton, WidgetInput};
use crate::widgets::primitives::{badge::BadgeWidget, support::activate_on_keyboard};

pub(super) fn handle_badge_input(
    badge: &mut BadgeWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<BadgeMessage> {
    if badge.common.state.disabled {
        badge.common.state.pressed = false;
        badge.state.armed = false;
        return None;
    }
    match input {
        WidgetInput::PointerMove { position } => {
            badge.common.state.hovered = bounds.contains(position);
            if badge.common.state.pressed {
                badge.state.armed = badge.common.state.hovered;
            }
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            badge.common.state.focused = true;
            badge.common.state.hovered = true;
            badge.common.state.pressed = true;
            badge.state.armed = true;
            None
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
        } => {
            let activated =
                badge.common.state.pressed && badge.state.armed && bounds.contains(position);
            badge.common.state.pressed = false;
            badge.common.state.hovered = bounds.contains(position);
            badge.state.armed = false;
            activated.then_some(BadgeMessage::Activate)
        }
        WidgetInput::FocusChanged(focused) => {
            badge.common.state.focused = focused;
            if !focused {
                badge.common.state.pressed = false;
                badge.state.armed = false;
            }
            None
        }
        WidgetInput::KeyPress(key) if badge.common.state.focused && activate_on_keyboard(key) => {
            Some(BadgeMessage::Activate)
        }
        _ => None,
    }
}
