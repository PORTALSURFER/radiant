//! Button pointer and keyboard interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{ButtonMessage, DragHandleMessage, PointerButton, WidgetInput};

use super::ButtonWidget;
use crate::widgets::primitives::support::activate_on_keyboard;

pub(super) fn handle_button_input(
    button: &mut ButtonWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<ButtonMessage> {
    if button.common.state.disabled {
        button.common.state.pressed = false;
        button.state.armed = false;
        return None;
    }
    match input {
        WidgetInput::PointerMove { position } => {
            button.common.state.hovered = bounds.contains(position);
            if button.common.state.pressed {
                button.state.armed = button.common.state.hovered;
                if button.props.drag {
                    let message = if button.state.dragged {
                        DragHandleMessage::Moved { position }
                    } else {
                        button.state.dragged = true;
                        DragHandleMessage::Started { position }
                    };
                    return Some(ButtonMessage::Drag(message));
                }
            }
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            button.common.state.focused = true;
            button.common.state.hovered = true;
            button.common.state.pressed = true;
            button.state.armed = true;
            button.state.dragged = false;
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Secondary,
            ..
        } if bounds.contains(position) && button.props.secondary_click => {
            button.common.state.hovered = true;
            Some(ButtonMessage::SecondaryActivate { position })
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => {
            if button.state.dragged {
                button.common.state.pressed = false;
                button.common.state.hovered = bounds.contains(position);
                button.state.armed = false;
                button.state.dragged = false;
                return Some(ButtonMessage::Drag(DragHandleMessage::Ended { position }));
            }
            let activated =
                button.common.state.pressed && button.state.armed && bounds.contains(position);
            button.common.state.pressed = false;
            button.common.state.hovered = bounds.contains(position);
            button.state.armed = false;
            button.state.dragged = false;
            activated.then_some(ButtonMessage::Activate)
        }
        WidgetInput::FocusChanged(focused) => {
            button.common.state.focused = focused;
            if !focused {
                button.common.state.pressed = false;
                button.state.armed = false;
                button.state.dragged = false;
            }
            None
        }
        WidgetInput::KeyPress(key) if button.common.state.focused && activate_on_keyboard(key) => {
            Some(ButtonMessage::Activate)
        }
        _ => None,
    }
}
