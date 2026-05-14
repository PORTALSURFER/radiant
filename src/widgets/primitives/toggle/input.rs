//! Toggle pointer and keyboard event routing.

use crate::gui::types::Rect;
use crate::widgets::interaction::{PointerButton, ToggleMessage, WidgetInput};

use super::ToggleWidget;
use crate::widgets::primitives::support::activate_on_keyboard;

pub(super) fn handle_toggle_input(
    toggle: &mut ToggleWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<ToggleMessage> {
    if toggle.common.state.disabled {
        toggle.common.state.pressed = false;
        toggle.state.armed = false;
        return None;
    }
    match input {
        WidgetInput::PointerMove { position } => {
            toggle.common.state.hovered = bounds.contains(position);
            if toggle.common.state.pressed {
                toggle.state.armed = toggle.common.state.hovered;
            }
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            toggle.common.state.focused = true;
            toggle.common.state.hovered = true;
            toggle.common.state.pressed = true;
            toggle.state.armed = true;
            None
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => {
            let should_toggle =
                toggle.common.state.pressed && toggle.state.armed && bounds.contains(position);
            toggle.common.state.pressed = false;
            toggle.common.state.hovered = bounds.contains(position);
            toggle.state.armed = false;
            should_toggle.then(|| toggle.toggle())
        }
        WidgetInput::FocusChanged(focused) => {
            toggle.common.state.focused = focused;
            if !focused {
                toggle.common.state.pressed = false;
                toggle.state.armed = false;
            }
            None
        }
        WidgetInput::KeyPress(key) if toggle.common.state.focused && activate_on_keyboard(key) => {
            Some(toggle.toggle())
        }
        _ => None,
    }
}
