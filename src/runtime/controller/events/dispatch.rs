use super::super::SurfaceRuntime;
use super::{Event, PointerClickOutcome};
use crate::{
    gui::types::Point,
    gui::{focus::FocusSurface, input::KeyPress},
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
    /// that only update runtime state, such as resize, focus clearing, or
    /// pointer-capture cancellation, return `None`.
    pub fn dispatch_event(&mut self, event: Event) -> Option<WidgetId> {
        if event_pointer_position(&event).is_some_and(|position| !position.is_finite()) {
            return None;
        }
        let target = match event {
            Event::Resize { viewport } => {
                self.set_viewport(viewport);
                None
            }
            Event::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                self.observe_pointer_position(position);
                self.dispatch_pointer_move_target_with_metadata(
                    position,
                    modifiers,
                    timestamp,
                    sequence_range,
                )
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
            Event::PointerCaptureCancelled => {
                self.cancel_pointer_capture();
                None
            }
            Event::KeyPress {
                key,
                modifiers,
                repeat,
                timestamp,
            } => {
                self.dispatch_key_press_outcome(key, modifiers, repeat, timestamp)
                    .0
            }
            Event::KeyRelease {
                key,
                modifiers,
                timestamp,
            } => {
                match self.dispatch_metadata_focused_key_release(Some(key), modifiers, timestamp) {
                    Some(route) => route.widget_id,
                    None => self.dispatch_focused_input(WidgetInput::key_release_with_metadata(
                        key, modifiers, timestamp,
                    )),
                }
            }
            Event::Character {
                character,
                timestamp,
            } => self.dispatch_focused_input(WidgetInput::character_with_timestamp(
                character, timestamp,
            )),
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
                sequence_range,
            } => {
                self.observe_pointer_position(position);
                self.wheel_or_scroll_at_with_metadata(
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                );
                None
            }
        };
        self.service_pending_current_surface_relayout();
        target
    }

    /// Dispatch a keyboard event and report whether the runtime consumes it.
    ///
    /// Embedded hosts must use this result for keyboard passthrough; the target
    /// returned by `dispatch_event` does not indicate consumption. Characters
    /// belong only to an enabled, editable text target. Non-keyboard events are
    /// rejected without dispatch. Custom widgets retain their declared focused
    /// key disposition, including consumption at an unchanged value boundary.
    pub fn dispatch_keyboard_event(&mut self, event: Event) -> bool {
        let consumed = match event {
            Event::KeyPress {
                key,
                modifiers,
                repeat,
                timestamp,
            } => {
                self.dispatch_key_press_outcome(key, modifiers, repeat, timestamp)
                    .1
            }
            Event::Character {
                character,
                timestamp,
            } => {
                let target = self.focused_text_input_id().filter(|id| {
                    self.is_authoritative_focus_target(*id)
                        && self.surface_widget(*id).is_some_and(|widget| {
                            let state = &widget.widget_object().common().state;
                            !state.disabled && !state.read_only
                        })
                });
                !character.is_control()
                    && target.is_some_and(|id| {
                        self.dispatch_input(
                            id,
                            WidgetInput::character_with_timestamp(character, timestamp),
                        )
                    })
            }
            Event::KeyRelease {
                key,
                modifiers,
                timestamp,
            } => self
                .dispatch_metadata_focused_key_release(Some(key), modifiers, timestamp)
                .is_some_and(|route| route.consumed),
            _ => false,
        };
        self.service_pending_current_surface_relayout();
        consumed
    }

    fn dispatch_key_press_outcome(
        &mut self,
        key: crate::widgets::WidgetKey,
        modifiers: crate::widgets::KeyboardModifiers,
        repeat: bool,
        timestamp: Option<crate::gui::input::InputTimestamp>,
    ) -> (Option<WidgetId>, bool) {
        let host_press = KeyPress {
            key: key.to_key_code(),
            command: if std::env::consts::OS == "macos" {
                modifiers.command
            } else {
                modifiers.command || modifiers.control
            },
            control: std::env::consts::OS == "macos" && modifiers.control,
            shift: modifiers.shift,
            alt: modifiers.alt,
        };
        match self.dispatch_metadata_focused_key_press(
            Some(host_press),
            Some(key),
            modifiers,
            timestamp,
            repeat,
            FocusSurface::None,
        ) {
            Some(route) => {
                let mut consumed = route.consumed;
                if route.fallback_eligible
                    && let (Some(widget_id), Some(key)) = (route.widget_id, Some(key))
                {
                    consumed |= self.scroll_keyboard_fallback(widget_id, key, timestamp);
                }
                (route.widget_id, consumed)
            }
            None => {
                let delivery = self.dispatch_focused_key_input(
                    WidgetInput::key_press_with_metadata(key, modifiers, repeat, timestamp),
                );
                if let Some(delivery) = delivery {
                    let mut consumed =
                        delivery.disposition == crate::widgets::FocusedKeyDisposition::Consumed;
                    if delivery.fallback_eligible
                        && delivery.disposition == crate::widgets::FocusedKeyDisposition::Unhandled
                    {
                        consumed |=
                            self.scroll_keyboard_fallback(delivery.widget_id, key, timestamp);
                    }
                    (Some(delivery.widget_id), consumed)
                } else {
                    (None, false)
                }
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

fn event_pointer_position(event: &Event) -> Option<Point> {
    match event {
        Event::PointerMove { position, .. }
        | Event::PointerPress { position, .. }
        | Event::PointerDoubleClick { position, .. }
        | Event::PointerRelease { position, .. }
        | Event::Scroll { position, .. } => Some(*position),
        _ => None,
    }
}
