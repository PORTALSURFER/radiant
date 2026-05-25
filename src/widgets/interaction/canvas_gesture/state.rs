use super::{
    event::CanvasGestureEvent,
    pointer::{canvas_pointer, point_delta},
};
use crate::{
    gui::types::{Rect, Vector2},
    widgets::interaction::WidgetInput,
};

mod active_press;

use active_press::ActiveCanvasPress;

/// Retained pointer gesture state for canvas-like custom widgets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasGestureState {
    active_press: Option<ActiveCanvasPress>,
}

impl CanvasGestureState {
    /// Build an idle gesture state.
    pub const fn new() -> Self {
        Self { active_press: None }
    }

    /// Return whether a pointer press is currently captured.
    pub const fn is_dragging(&self) -> bool {
        self.active_press.is_some()
    }

    /// Clear any active pointer capture.
    pub fn cancel(&mut self) {
        self.active_press = None;
    }

    /// Convert one raw widget input into a canvas gesture event.
    pub fn handle_input(
        &mut self,
        bounds: Rect,
        input: &WidgetInput,
    ) -> Option<CanvasGestureEvent> {
        match input {
            WidgetInput::PointerMove { position } => {
                let pointer = canvas_pointer(bounds, *position)?;
                Some(match self.active_press {
                    Some(active) => CanvasGestureEvent::Drag {
                        pointer,
                        origin: active.origin,
                        delta: point_delta(active.origin.position, *position),
                        button: active.button,
                        modifiers: active.modifiers,
                    },
                    None => CanvasGestureEvent::Hover(pointer),
                })
            }
            WidgetInput::PointerPress {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                self.active_press = Some(ActiveCanvasPress::new(pointer, *button, *modifiers));
                Some(CanvasGestureEvent::Press {
                    pointer,
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerDoubleClick {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                Some(CanvasGestureEvent::DoubleClick {
                    pointer,
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerRelease {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                let active = self.active_press.take();
                Some(CanvasGestureEvent::Release {
                    pointer,
                    origin: active.map_or(pointer, |active| active.origin),
                    delta: active.map_or(Vector2::default(), |active| {
                        point_delta(active.origin.position, *position)
                    }),
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::PointerDrop {
                position,
                button,
                modifiers,
            } => {
                let pointer = canvas_pointer(bounds, *position)?;
                let active = self.active_press.take();
                Some(CanvasGestureEvent::Drop {
                    pointer,
                    origin: active.map(|active| active.origin),
                    button: *button,
                    modifiers: *modifiers,
                })
            }
            WidgetInput::Wheel {
                position, delta, ..
            } => Some(CanvasGestureEvent::Wheel {
                pointer: canvas_pointer(bounds, *position)?,
                delta: *delta,
            }),
            WidgetInput::FocusChanged(focused) => {
                if !focused {
                    self.cancel();
                }
                Some(CanvasGestureEvent::FocusChanged(*focused))
            }
            WidgetInput::PointerModifiersChanged { .. }
            | WidgetInput::KeyPress(_)
            | WidgetInput::Character(_)
            | WidgetInput::TextEdit(_) => None,
        }
    }
}

#[cfg(test)]
mod tests;
