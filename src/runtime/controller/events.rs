use super::{FocusTraversal, SurfaceRuntime};
use crate::{
    gui::types::{Point, Vector2},
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers, WidgetId, WidgetInput, WidgetKey},
};

mod pointer;

/// Routing summary for one pointer-move event.
///
/// Backend adapters that distinguish full scene rebuilds from paint-only
/// overlays can use this instead of [`SurfaceRuntime::dispatch_event`] for
/// pointer motion. The outcome drains the runtime repaint/exit flags observed
/// during the route so callers can make one redraw decision without peeking at
/// controller internals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerMoveOutcome {
    /// Widget targeted by the pointer move, when one was hit.
    pub target: Option<WidgetId>,
    /// Whether hover ownership changed during this route.
    pub hover_changed: bool,
    /// Whether a widget currently owns pointer capture.
    pub pointer_captured: bool,
    /// Whether the base surface or Vello scene should be rebuilt.
    pub repaint_requested: bool,
    /// Whether a cached-scene overlay redraw is enough.
    pub paint_only_requested: bool,
    /// Whether routing requested runtime shutdown.
    pub exit_requested: bool,
}

impl PointerMoveOutcome {
    /// Return whether a projected widget received the pointer move.
    pub fn routed(self) -> bool {
        self.target.is_some()
    }

    /// Return whether a backend should redraw the frame.
    pub fn needs_redraw(self) -> bool {
        self.needs_scene_rebuild() || self.paint_only_requested
    }

    /// Return whether the cached scene is stale.
    pub fn needs_scene_rebuild(self) -> bool {
        self.hover_changed || self.repaint_requested
    }
}

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
        /// Modifier state when the press started.
        modifiers: PointerModifiers,
    },
    /// Pointer button was pressed twice in quick succession.
    PointerDoubleClick {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that completed the double-click.
        button: PointerButton,
        /// Modifier state when the double-click completed.
        modifiers: PointerModifiers,
    },
    /// Pointer press ended at the given surface position.
    PointerRelease {
        /// Pointer position in surface logical coordinates.
        position: Point,
        /// Pointer button that ended the press.
        button: PointerButton,
        /// Modifier state when the press ended.
        modifiers: PointerModifiers,
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
            Event::PointerMove { position } => self.dispatch_pointer_move_target(position),
            Event::PointerPress {
                position,
                button,
                modifiers,
            } => self.dispatch_pointer_press_event(position, button, modifiers),
            Event::PointerDoubleClick {
                position,
                button,
                modifiers,
            } => self.dispatch_pointer_double_click_event(position, button, modifiers),
            Event::PointerRelease {
                position,
                button,
                modifiers,
            } => self.dispatch_pointer_release_event(position, button, modifiers),
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
