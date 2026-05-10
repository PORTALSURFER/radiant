//! Drag-handle pointer interaction behavior.

use crate::gui::types::Rect;
use crate::widgets::interaction::{DragHandleMessage, PointerButton, WidgetInput};
use crate::widgets::primitives::drag_handle::DragHandleWidget;

pub(super) fn handle_drag_handle_input(
    handle: &mut DragHandleWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<DragHandleMessage> {
    if handle.common.state.disabled {
        return None;
    }

    match input {
        WidgetInput::PointerMove { position } => {
            handle.common.state.hovered = bounds.contains(position);
            handle
                .common
                .state
                .pressed
                .then_some(DragHandleMessage::Moved { position })
        }
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
        } if bounds.contains(position) => {
            handle.common.state.pressed = true;
            handle.common.state.active = true;
            Some(DragHandleMessage::Started { position })
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
        } => {
            handle.common.state.pressed = false;
            handle.common.state.active = false;
            Some(DragHandleMessage::Ended { position })
        }
        WidgetInput::FocusChanged(focused) => {
            handle.common.state.focused = focused;
            None
        }
        _ => None,
    }
}
