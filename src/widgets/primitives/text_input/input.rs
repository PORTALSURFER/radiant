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
        WidgetInput::PointerMove { position, .. } => {
            text_input.common.state.hovered = bounds.contains(position);
            if text_input.common.state.pressed {
                let caret = text_input
                    .take_native_pointer_caret()
                    .unwrap_or_else(|| caret_for_pointer_x(bounds, position.x));
                text_input.set_caret(caret, true);
            } else {
                let _ = text_input.take_native_pointer_caret();
            }
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            text_input.common.state.focused = true;
            text_input.common.state.hovered = true;
            text_input.common.state.pressed = true;
            let caret = text_input
                .take_native_pointer_caret()
                .unwrap_or_else(|| caret_for_pointer_x(bounds, position.x));
            text_input.set_caret(caret, false);
            None
        }
        WidgetInput::PointerDoubleClick {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            text_input.common.state.focused = true;
            text_input.common.state.hovered = true;
            text_input.common.state.pressed = false;
            let caret = text_input
                .take_native_pointer_caret()
                .unwrap_or_else(|| caret_for_pointer_x(bounds, position.x));
            text_input.select_word_at(caret);
            None
        }
        WidgetInput::PointerRelease {
            button: PointerButton::Primary,
            ..
        } => {
            let _ = text_input.take_native_pointer_caret();
            text_input.common.state.pressed = false;
            None
        }
        WidgetInput::FocusChanged(focused) => {
            text_input.common.state.focused = focused;
            if !focused {
                text_input.cancel_composition();
            }
            None
        }
        WidgetInput::Character { character: ch, .. }
            if text_input.accepts_editing_input() && !ch.is_control() =>
        {
            text_input.insert_text(ch.encode_utf8(&mut [0; 4]))
        }
        WidgetInput::KeyPress { key, .. } if text_input.accepts_editing_input() => {
            text_input.handle_key_input(key)
        }
        WidgetInput::TextEdit { command, .. } if text_input.accepts_editing_input() => {
            text_input.handle_text_edit(command)
        }
        _ => None,
    }
}
