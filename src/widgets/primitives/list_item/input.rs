//! List-item pointer and keyboard invocation behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{ListItemMessage, PointerButton, WidgetInput};
use crate::widgets::primitives::{list_item::ListItemWidget, support::activate_on_keyboard};

pub(super) fn handle_list_item_input(
    item: &mut ListItemWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<ListItemMessage> {
    if item.common.state.disabled {
        return None;
    }

    match input {
        WidgetInput::PointerMove { position } => {
            item.common.state.hovered = bounds.contains(position);
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            item.common.state.pressed = true;
            None
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
        } => {
            let was_pressed = item.common.state.pressed;
            item.common.state.pressed = false;
            (was_pressed && bounds.contains(position)).then_some(ListItemMessage::Invoked)
        }
        WidgetInput::FocusChanged(focused) => {
            item.common.state.focused = focused;
            None
        }
        WidgetInput::KeyPress(key) if item.common.state.focused && activate_on_keyboard(key) => {
            Some(ListItemMessage::Invoked)
        }
        _ => None,
    }
}
