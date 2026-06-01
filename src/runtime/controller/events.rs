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

/// Routing summary for a synthetic pointer click dispatched through the normal
/// backend-neutral event path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerClickOutcome {
    /// Widget targeted by the press event, when one was hit.
    pub press_target: Option<WidgetId>,
    /// Widget targeted by the matching release event, when one was hit.
    pub release_target: Option<WidgetId>,
}

impl PointerClickOutcome {
    /// Return whether the press and release both routed to the same widget.
    pub fn completed_on_same_widget(self) -> bool {
        self.press_target.is_some() && self.press_target == self.release_target
    }

    /// Return the widget that received both press and release, when the click
    /// completed on one widget.
    pub fn completed_widget(self) -> Option<WidgetId> {
        self.completed_on_same_widget()
            .then_some(self.press_target)
            .flatten()
    }
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
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
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
            Event::PointerMove { position } => self.dispatch_pointer_move_target(position).target,
            Event::PointerModifiersChanged { modifiers } => {
                self.dispatch_pointer_modifiers_changed(modifiers)
            }
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

    /// Route a pointer press followed by a matching release at the same point.
    ///
    /// This is a convenience for tests, embedded hosts, and automation paths
    /// that need to exercise the same click routing as native backends without
    /// repeating the press/release event boilerplate.
    pub fn dispatch_pointer_click(
        &mut self,
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> PointerClickOutcome {
        let press_target = self.dispatch_event(Event::PointerPress {
            position,
            button,
            modifiers,
        });
        let release_target = self.dispatch_event(Event::PointerRelease {
            position,
            button,
            modifiers,
        });
        PointerClickOutcome {
            press_target,
            release_target,
        }
    }

    /// Route a primary-button click with no keyboard modifiers.
    pub fn dispatch_primary_click(&mut self, position: Point) -> PointerClickOutcome {
        self.dispatch_pointer_click(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }
}
