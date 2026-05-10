//! Selectable pointer and keyboard interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{PointerButton, SelectableMessage, WidgetInput};
use crate::widgets::primitives::{selectable::SelectableWidget, support::activate_on_keyboard};

pub(super) fn handle_selectable_input(
    selectable: &mut SelectableWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<SelectableMessage> {
    if selectable.common.state.disabled {
        return None;
    }

    match input {
        WidgetInput::PointerMove { position } => {
            selectable.common.state.hovered = bounds.contains(position);
            None
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            selectable.common.state.pressed = true;
            None
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
        } => {
            let was_pressed = selectable.common.state.pressed;
            selectable.common.state.pressed = false;
            (was_pressed && bounds.contains(position)).then(|| toggle_selected(selectable))
        }
        WidgetInput::FocusChanged(focused) => {
            selectable.common.state.focused = focused;
            None
        }
        WidgetInput::KeyPress(key)
            if selectable.common.state.focused && activate_on_keyboard(key) =>
        {
            Some(toggle_selected(selectable))
        }
        _ => None,
    }
}

fn toggle_selected(selectable: &mut SelectableWidget) -> SelectableMessage {
    selectable.common.state.selected = !selectable.common.state.selected;
    SelectableMessage::SelectionChanged {
        selected: selectable.common.state.selected,
    }
}
