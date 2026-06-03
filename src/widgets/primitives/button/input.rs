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
        button.common.state.active = false;
        button.state.armed = false;
        button.state.press_position = None;
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
                        button.common.state.active = true;
                        DragHandleMessage::Started {
                            position: button.state.press_position.unwrap_or(position),
                        }
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
            button.common.state.active = false;
            button.state.armed = true;
            button.state.dragged = false;
            button.state.press_position = Some(position);
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
            if button.state.dragged || (button.props.drag && button.common.state.active) {
                button.common.state.pressed = false;
                button.common.state.active = false;
                button.common.state.hovered = bounds.contains(position);
                button.state.armed = false;
                button.state.dragged = false;
                button.state.press_position = None;
                return Some(ButtonMessage::Drag(DragHandleMessage::Ended { position }));
            }
            let activated =
                button.common.state.pressed && button.state.armed && bounds.contains(position);
            button.common.state.pressed = false;
            button.common.state.active = false;
            button.common.state.hovered = bounds.contains(position);
            button.state.armed = false;
            button.state.dragged = false;
            button.state.press_position = None;
            activated.then_some(ButtonMessage::Activate)
        }
        WidgetInput::FocusChanged(focused) => {
            let cancel_drag = !focused
                && button.props.drag
                && (button.state.dragged || button.common.state.active)
                && button.state.press_position.is_some();
            button.common.state.focused = focused;
            if !focused {
                let position = button.state.press_position.unwrap_or_default();
                button.common.state.pressed = false;
                button.common.state.active = false;
                button.state.armed = false;
                button.state.dragged = false;
                button.state.press_position = None;
                if cancel_drag {
                    return Some(ButtonMessage::Drag(DragHandleMessage::Cancelled {
                        position,
                    }));
                }
            }
            None
        }
        WidgetInput::KeyPress(key) if button.common.state.focused && activate_on_keyboard(key) => {
            Some(ButtonMessage::Activate)
        }
        _ => None,
    }
}
