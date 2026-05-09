//! Text-input pointer, keyboard, and text-edit event routing.

use crate::gui::types::Rect;
use crate::widgets::interaction::{PointerButton, TextInputMessage, WidgetInput};

use super::TextInputWidget;
use super::editing_ops::caret_for_pointer_x;

pub(super) fn handle_text_input(
    text_input: &mut TextInputWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<TextInputMessage> {
    match input {
        WidgetInput::PointerMove { position } => {
            text_input.common.state.hovered = bounds.contains(position);
            if text_input.common.state.pressed {
                text_input.set_caret(caret_for_pointer_x(bounds, position.x), true);
            }
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            text_input.common.state.focused = true;
            text_input.common.state.hovered = true;
            text_input.common.state.pressed = true;
            text_input.set_caret(caret_for_pointer_x(bounds, position.x), false);
            None
        }
        WidgetInput::PointerRelease {
            button: PointerButton::Primary,
            ..
        } => {
            text_input.common.state.pressed = false;
            None
        }
        WidgetInput::FocusChanged(focused) => {
            text_input.common.state.focused = focused;
            None
        }
        WidgetInput::Character(ch)
            if text_input.common.state.focused
                && !text_input.common.state.disabled
                && !text_input.common.state.read_only
                && !ch.is_control() =>
        {
            text_input.insert_text(ch.encode_utf8(&mut [0; 4]))
        }
        WidgetInput::KeyPress(key)
            if text_input.common.state.focused
                && !text_input.common.state.disabled
                && !text_input.common.state.read_only =>
        {
            text_input.handle_key_input(key)
        }
        WidgetInput::TextEdit(command)
            if text_input.common.state.focused
                && !text_input.common.state.disabled
                && !text_input.common.state.read_only =>
        {
            text_input.handle_text_edit(command)
        }
        _ => None,
    }
}
