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
            ..
        } if bounds.contains(position) => {
            handle.common.state.pressed = true;
            handle.common.state.active = true;
            Some(DragHandleMessage::Started { position })
        }
        WidgetInput::PointerDoubleClick {
            position,
            button: PointerButton::Primary,
            ..
        } if bounds.contains(position) => {
            handle.common.state.hovered = true;
            handle.common.state.pressed = false;
            handle.common.state.active = false;
            Some(DragHandleMessage::DoubleActivate { position })
        }
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            ..
        } => {
            handle.common.state.pressed = false;
            handle.common.state.active = false;
            Some(DragHandleMessage::Ended { position })
        }
        WidgetInput::FocusChanged(focused) => {
            let cancel_drag = !focused && handle.common.state.active;
            handle.common.state.focused = focused;
            if cancel_drag {
                handle.common.state.pressed = false;
                handle.common.state.active = false;
                return Some(DragHandleMessage::Cancelled {
                    position: bounds.center(),
                });
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Vector2},
        widgets::WidgetSizing,
    };

    #[test]
    fn drag_handle_double_click_emits_double_activate() {
        let mut handle = DragHandleWidget::new(7, WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
        let bounds = Rect::from_size(24.0, 16.0);
        let position = Point::new(8.0, 6.0);

        let message = handle_drag_handle_input(
            &mut handle,
            bounds,
            WidgetInput::primary_double_click(position),
        );

        assert_eq!(
            message,
            Some(DragHandleMessage::DoubleActivate { position })
        );
        assert!(!handle.common.state.pressed);
        assert!(!handle.common.state.active);
    }

    #[test]
    fn drag_handle_focus_loss_cancels_active_drag() {
        let mut handle = DragHandleWidget::new(8, WidgetSizing::fixed(Vector2::new(24.0, 16.0)));
        let bounds = Rect::from_size(24.0, 16.0);

        assert_eq!(
            handle_drag_handle_input(
                &mut handle,
                bounds,
                WidgetInput::primary_press(Point::new(8.0, 6.0)),
            ),
            Some(DragHandleMessage::Started {
                position: Point::new(8.0, 6.0)
            })
        );
        assert_eq!(
            handle_drag_handle_input(&mut handle, bounds, WidgetInput::FocusChanged(false)),
            Some(DragHandleMessage::Cancelled {
                position: bounds.center()
            })
        );
        assert!(!handle.common.state.pressed);
        assert!(!handle.common.state.active);
    }
}
