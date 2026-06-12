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

    /// Route a secondary-button click with no keyboard modifiers.
    pub fn dispatch_secondary_click(&mut self, position: Point) -> PointerClickOutcome {
        self.dispatch_pointer_click(
            position,
            PointerButton::Secondary,
            PointerModifiers::default(),
        )
    }
}
