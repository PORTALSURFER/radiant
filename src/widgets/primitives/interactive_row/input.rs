//! Backend-neutral input behavior for interactive rows.

use super::InteractiveRowWidget;
use crate::{
    gui::types::{Point, Rect},
    widgets::interaction::{
        DragHandleMessage, InteractiveRowMessage, PointerButton, WidgetInput, WidgetKey,
    },
};

const ROW_DRAG_START_DISTANCE: f32 = 7.0;
const ROW_DRAG_START_DISTANCE_SQUARED: f32 = ROW_DRAG_START_DISTANCE * ROW_DRAG_START_DISTANCE;

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
                    if !self.dragged && !drag_start_threshold_met(self.pressed_position, position) {
                        return None;
                    }
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
                    if self.props.clear_drop_on_hover {
                        return Some(InteractiveRowMessage::ClearDropTarget { position });
                    }
                    return Some(InteractiveRowMessage::HoverDropTarget { position });
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
                self.pressed_position = Some(position);
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
                self.pressed_position = None;
                self.dragged = false;
                Some(InteractiveRowMessage::DoubleActivate)
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
            } => {
                if self.props.droppable
                    && self.props.drag_active
                    && !self.props.drag_source
                    && bounds.contains(position)
                {
                    self.common.state.pressed = false;
                    self.common.state.hovered = true;
                    self.pressed_position = None;
                    self.dragged = false;
                    return Some(InteractiveRowMessage::Drop);
                }
                let activated =
                    self.common.state.pressed && !self.dragged && bounds.contains(position);
                let dragged = self.props.drag_source || (self.common.state.pressed && self.dragged);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                self.pressed_position = None;
                self.dragged = false;
                if dragged {
                    return Some(InteractiveRowMessage::Drag(DragHandleMessage::Ended {
                        position,
                    }));
                }
                if !activated {
                    return None;
                }
                if self.props.activation_modifiers {
                    Some(InteractiveRowMessage::ActivateWithModifiers { modifiers })
                } else {
                    Some(InteractiveRowMessage::Activate)
                }
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
                if !focused && !self.dragged && !self.props.drag_source {
                    self.common.state.pressed = false;
                    self.pressed_position = None;
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
                    self.pressed_position = None;
                    self.dragged = false;
                }
                None
            }
        }
    }
}

fn drag_start_threshold_met(start: Option<Point>, current: Point) -> bool {
    let Some(start) = start else {
        return true;
    };
    let dx = current.x - start.x;
    let dy = current.y - start.y;
    (dx * dx + dy * dy) >= ROW_DRAG_START_DISTANCE_SQUARED
}
