use super::{
    CpuFrameObservationOwner, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    GenericRouteOutcome, key_code_from_winit, keyboard_modifiers_from_winit, keypress_from_input,
};
use crate::gui::input::{InputTimestamp, KeyCode, KeyPress};
use crate::{runtime::RuntimeBridge, widgets::WidgetKey};
use std::time::Instant;
use winit::{
    event::{ElementState, KeyEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey, PhysicalKey},
};

mod repeat;
mod text_edit;

use repeat::should_route_keypress;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_keyboard_event(&mut self, event_loop: &ActiveEventLoop, event: KeyEvent) {
        self.handle_keyboard_event_inner(event_loop, event, None, None);
    }

    pub(super) fn handle_keyboard_event_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyEvent,
        adapter: &mut GenericNativeAdapterOwner,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        self.handle_keyboard_event_inner(event_loop, event, Some(adapter), observation);
    }

    fn handle_keyboard_event_inner(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: KeyEvent,
        mut adapter: Option<&mut GenericNativeAdapterOwner>,
        mut observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        if event.state == ElementState::Released {
            if let Some(outcome) = self.route_native_key_release(event.physical_key) {
                self.route_keyboard_outcome(event_loop, outcome, adapter, observation);
            }
            return;
        }
        if event.state != ElementState::Pressed {
            return;
        }
        self.sync_runtime_pointer_from_native_cursor();
        let repeat = event.repeat;
        let mut repeat_accepted = !repeat;
        let mut route_outcome = GenericRouteOutcome::default();
        let logical_text = keyboard_event_text(&event);
        let physical_key = match event.physical_key {
            PhysicalKey::Code(code) => key_code_from_winit(code),
            PhysicalKey::Unidentified(_) => None,
        };
        let timestamp = Some(InputTimestamp::capture());
        let widget_modifiers = keyboard_modifiers_from_winit(self.input.modifiers);
        if let Some(outcome) = self.core.route_metadata_key_press_with_timestamp(
            physical_key.map(|key| keypress_from_input(key, self.input.modifiers)),
            physical_key.and_then(WidgetKey::from_key_code),
            widget_modifiers,
            timestamp,
            repeat,
        ) {
            self.route_keyboard_outcome(
                event_loop,
                outcome,
                adapter.as_deref_mut(),
                observation.as_deref_mut(),
            );
            return;
        }
        if let Some(key) = physical_key {
            let allow_text_deletion_repeat = repeat
                && self.core.has_focused_text_input()
                && !self.input.modifiers.alt_key()
                && matches!(key, KeyCode::Backspace | KeyCode::Delete);
            if !should_route_keypress(
                key,
                repeat,
                allow_text_deletion_repeat,
                &mut self.input.last_navigation_key_repeat,
                Instant::now(),
            ) {
                return;
            }
            repeat_accepted = true;
            Some(key)
        } else {
            None
        };
        if !repeat_accepted {
            return;
        }
        if let Some(key) = physical_key {
            if self.route_text_input_shortcut(key, timestamp, &mut route_outcome) {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
            if self.route_text_navigation_key(key, timestamp, &mut route_outcome) {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
            if self.route_focused_widget_preempting_shortcut_key(
                key,
                timestamp,
                repeat,
                &mut route_outcome,
            ) {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
            if self.route_space_text_input(key, timestamp, &mut route_outcome) {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
            if self.route_focused_text_input_before_shortcuts(
                key,
                logical_text,
                timestamp,
                repeat,
                &mut route_outcome,
            ) {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
            let outcome = self.core.route_key_press_with_timestamp(
                keypress_from_input(key, self.input.modifiers),
                WidgetKey::from_key_code(key),
                widget_modifiers,
                timestamp,
                repeat,
            );
            route_outcome.merge(outcome);
        }
        if !route_outcome.routed
            && !self.core.has_focused_text_input()
            && let Some(press) = logical_shortcut_keypress_from_text(logical_text)
        {
            let outcome = self.core.route_key_press_with_timestamp(
                press,
                None,
                widget_modifiers,
                timestamp,
                false,
            );
            route_outcome.merge(outcome);
            if route_outcome.routed {
                self.route_keyboard_outcome(
                    event_loop,
                    route_outcome,
                    adapter.as_deref_mut(),
                    observation.as_deref_mut(),
                );
                return;
            }
        }
        if let Some(text) = event.text.as_ref() {
            self.route_text_input_after_unhandled_keypress(text, timestamp, &mut route_outcome);
        } else if matches!(event.logical_key, Key::Named(NamedKey::Space)) {
            self.route_text_input_after_unhandled_keypress(" ", timestamp, &mut route_outcome);
        } else if let Key::Character(text) = &event.logical_key {
            self.route_text_input_after_unhandled_keypress(
                text.as_str(),
                timestamp,
                &mut route_outcome,
            );
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_widget_key_with_metadata(
                WidgetKey::Backspace,
                widget_modifiers,
                repeat,
                timestamp,
            );
            route_outcome.merge(outcome);
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_widget_key_with_metadata(
                WidgetKey::Delete,
                widget_modifiers,
                repeat,
                timestamp,
            );
            route_outcome.merge(outcome);
        }
        self.route_keyboard_outcome(event_loop, route_outcome, adapter, observation);
    }

    pub(in crate::gui_runtime::native_vello) fn route_native_key_release(
        &mut self,
        physical_key: PhysicalKey,
    ) -> Option<GenericRouteOutcome> {
        let PhysicalKey::Code(code) = physical_key else {
            return None;
        };
        let modifiers = keyboard_modifiers_from_winit(self.input.modifiers);
        let key = key_code_from_winit(code);
        let widget_key = key.and_then(WidgetKey::from_key_code);
        let timestamp = Some(InputTimestamp::capture());
        match widget_key {
            Some(widget_key) => Some(
                self.core
                    .route_key_release_with_metadata(widget_key, modifiers, timestamp),
            ),
            None => self
                .core
                .route_metadata_key_release_with_metadata(None, modifiers, timestamp),
        }
    }

    fn route_keyboard_outcome(
        &mut self,
        event_loop: &ActiveEventLoop,
        outcome: GenericRouteOutcome,
        adapter: Option<&mut GenericNativeAdapterOwner>,
        observation: Option<&mut CpuFrameObservationOwner<'_>>,
    ) {
        if let Some(adapter) = adapter {
            self.handle_route_outcome_with_adapter(event_loop, outcome, adapter, observation);
        } else {
            self.handle_route_outcome(event_loop, outcome);
        }
    }

    pub(super) fn sync_runtime_pointer_from_native_cursor(&mut self) {
        self.core
            .set_current_pointer_position(self.input.last_cursor);
    }
}

fn keyboard_event_text(event: &KeyEvent) -> Option<&str> {
    event.text.as_ref().map(|text| text.as_str()).or_else(|| {
        if let Key::Character(text) = &event.logical_key {
            Some(text.as_str())
        } else {
            None
        }
    })
}

fn logical_shortcut_keypress_from_text(text: Option<&str>) -> Option<KeyPress> {
    Some(KeyPress::new(match text? {
        "[" => KeyCode::OpenBracket,
        "]" => KeyCode::CloseBracket,
        _ => return None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_shortcut_keypress_from_text_maps_bracket_characters() {
        assert_eq!(
            logical_shortcut_keypress_from_text(Some("[")),
            Some(KeyPress::new(KeyCode::OpenBracket))
        );
        assert_eq!(
            logical_shortcut_keypress_from_text(Some("]")),
            Some(KeyPress::new(KeyCode::CloseBracket))
        );
    }

    #[test]
    fn logical_shortcut_keypress_from_text_ignores_non_exact_bracket_text() {
        assert_eq!(logical_shortcut_keypress_from_text(Some("{")), None);
        assert_eq!(logical_shortcut_keypress_from_text(Some("[]")), None);
        assert_eq!(logical_shortcut_keypress_from_text(None), None);
    }
}
