use super::{FocusTraversal, SurfaceRuntime};
use crate::{
    gui::types::{Point, Vector2},
    runtime::RuntimeBridge,
    widgets::{PointerButton, WidgetId, WidgetInput, WidgetKey},
};

/// Backend-neutral runtime event routed through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Viewport size changed and layout should be recomputed.
    Resize {
        /// New logical viewport size.
        viewport: Vector2,
    },
    /// Pointer hover moved across the surface.
    PointerMove {
        /// Pointer position in surface logical coordinates.
        position: Point,
    },
    /// Pointer press started at the given surface position.
    PointerPress {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that started the press.
        button: PointerButton,
    },
    /// Pointer press ended at the given surface position.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that ended the press.
        button: PointerButton,
    },
    /// One non-text key intent should route to the focused widget.
    KeyPress(WidgetKey),
    /// One printable character should route to the focused widget.
    Character(char),
    /// Move keyboard focus in declarative tree order.
    TraverseFocus(FocusTraversal),
    /// Clear current runtime focus ownership.
    ClearFocus,
    /// Scroll the scrollable container under the pointer by logical pixels.
    Scroll {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
    },
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Route one backend-neutral runtime event.
    ///
    /// Returns the targeted widget id when the event routes to a widget. Events
    /// that only update runtime state, such as resize or focus clearing, return
    /// `None`.
    pub fn dispatch_event(&mut self, event: Event) -> Option<WidgetId> {
        match event {
            Event::Resize { viewport } => {
                self.set_viewport(viewport);
                None
            }
            Event::PointerMove { position } => self.dispatch_pointer_move(position),
            Event::PointerPress { position, button } => {
                let Some(widget_id) = self.widget_at(position) else {
                    self.pointer_capture = None;
                    self.pointer_capture_state = None;
                    self.clear_focus();
                    return None;
                };
                self.pointer_capture = Some(widget_id);
                self.dispatch_input_at(position, WidgetInput::PointerPress { position, button })
            }
            Event::PointerRelease { position, button } => {
                let widget_id = self
                    .pointer_capture
                    .take()
                    .or_else(|| self.widget_at(position))?;
                self.pointer_capture_state = None;
                self.dispatch_input(widget_id, WidgetInput::PointerRelease { position, button })
                    .then_some(widget_id)
            }
            Event::KeyPress(key) => self.dispatch_focused_input(WidgetInput::KeyPress(key)),
            Event::Character(character) => {
                self.dispatch_focused_input(WidgetInput::Character(character))
            }
            Event::TraverseFocus(direction) => self.traverse_focus(direction),
            Event::ClearFocus => {
                self.clear_focus();
                None
            }
            Event::Scroll { position, delta } => {
                self.wheel_or_scroll_at(position, delta);
                None
            }
        }
    }
}
