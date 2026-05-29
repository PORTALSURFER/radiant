//! Backend-neutral input behavior for interactive rows.

use super::InteractiveRowWidget;
use crate::{
    gui::types::Rect,
    widgets::interaction::{
        DragHandleMessage, InteractiveRowMessage, PointerButton, WidgetInput, WidgetKey,
    },
};

impl InteractiveRowWidget {
    /// Route one backend-neutral interaction into the row.
    pub fn handle_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<InteractiveRowMessage> {
        match input {
            WidgetInput::PointerMove { position } => {
                if self.props.suppress_hover
                    || (self.props.drag_active && !self.props.drag_source && !self.props.droppable)
                {
                    self.common.state.hovered = false;
                    return None;
                }
                self.common.state.hovered = bounds.contains(position);
                if self.props.drag_source {
                    if self.props.drag_source_motion {
                        return Some(InteractiveRowMessage::Drag(DragHandleMessage::Moved {
                            position,
                        }));
                    }
                    return None;
                }
                if self.common.state.pressed && self.props.draggable {
                    let message = if self.dragged {
                        DragHandleMessage::Moved { position }
                    } else {
                        self.dragged = true;
                        DragHandleMessage::Started { position }
                    };
                    return Some(InteractiveRowMessage::Drag(message));
                }
                if self.common.state.hovered
                    && self.props.droppable
                    && self.props.drag_active
                    && self.props.drop_hover
                {
                    return Some(InteractiveRowMessage::HoverDropTarget);
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.dragged = false;
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                Some(InteractiveRowMessage::SecondaryActivate { position })
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = false;
                self.dragged = false;
                Some(InteractiveRowMessage::DoubleActivate)
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                if self.props.droppable
                    && self.props.drag_active
                    && !self.props.drag_source
                    && bounds.contains(position)
                {
                    self.common.state.pressed = false;
                    self.common.state.hovered = true;
                    self.dragged = false;
                    return Some(InteractiveRowMessage::Drop);
                }
                let activated =
                    self.common.state.pressed && !self.dragged && bounds.contains(position);
                let dragged = self.props.drag_source || (self.common.state.pressed && self.dragged);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.dragged = false;
                if dragged {
                    return Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
                        position,
                    }));
                }
                activated.then_some(InteractiveRowMessage::Activate)
            }
            WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } if self.props.droppable && bounds.contains(position) => {
                Some(InteractiveRowMessage::Drop)
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
            WidgetInput::KeyPress(WidgetKey::Enter | WidgetKey::Space)
                if self.common.state.focused =>
            {
                Some(InteractiveRowMessage::Activate)
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                    self.dragged = false;
                }
                None
            }
        }
    }
}
