use super::super::SurfaceRuntime;
use super::{Event, PointerClickOutcome};
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers, WidgetId, WidgetInput},
};

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
            Event::PointerMove {
                position,
                modifiers,
                timestamp,
            } => {
                self.observe_pointer_position(position);
                self.dispatch_pointer_move_target_with_metadata(position, modifiers, timestamp)
                    .target
            }
            Event::PointerModifiersChanged {
                modifiers,
                timestamp,
            } => self.dispatch_pointer_modifiers_changed(modifiers, timestamp),
            Event::PointerPress {
                position,
                button,
                modifiers,
                timestamp,
            } => {
                self.observe_pointer_position(position);
                self.dispatch_pointer_press_event(position, button, modifiers, timestamp)
            }
            Event::PointerDoubleClick {
                position,
                button,
                modifiers,
                timestamp,
            } => {
                self.observe_pointer_position(position);
                self.dispatch_pointer_double_click_event(position, button, modifiers, timestamp)
            }
            Event::PointerRelease {
                position,
                button,
                modifiers,
                timestamp,
            } => {
                self.observe_pointer_position(position);
                self.dispatch_pointer_release_event(position, button, modifiers, timestamp)
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
            Event::Scroll {
                position,
                delta,
                modifiers,
                timestamp,
            } => {
                self.observe_pointer_position(position);
                self.wheel_or_scroll_at_with_metadata(position, delta, modifiers, timestamp);
                None
            }
        }
    }

    fn observe_pointer_position(&mut self, position: Point) {
        self.interaction.pointer.current_position = Some(position);
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
            timestamp: None,
        });
        let release_target = self.dispatch_event(Event::PointerRelease {
            position,
            button,
            modifiers,
            timestamp: None,
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

    /// Route a secondary-button click with no keyboard modifiers.
    pub fn dispatch_secondary_click(&mut self, position: Point) -> PointerClickOutcome {
        self.dispatch_pointer_click(
            position,
            PointerButton::Secondary,
            PointerModifiers::default(),
        )
    }
}
