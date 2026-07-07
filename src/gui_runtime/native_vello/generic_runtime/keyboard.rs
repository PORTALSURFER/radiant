use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, key_code_from_winit, keypress_from_input,
};
use crate::gui::input::{KeyCode, KeyPress};
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
        if event.state != ElementState::Pressed {
            return;
        }
        self.sync_runtime_pointer_from_native_cursor();
        let repeat = event.repeat;
        let mut repeat_accepted = !repeat;
        let mut route_outcome = GenericRouteOutcome::default();
        let logical_text = keyboard_event_text(&event);
        if let PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = key_code_from_winit(code)
        {
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
            if self.route_text_input_shortcut(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_text_navigation_key(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_focused_widget_preempting_shortcut_key(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_space_text_input(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_focused_text_input_before_shortcuts(key, logical_text, &mut route_outcome)
            {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            let outcome = self.core.route_key_press(
                keypress_from_input(key, self.input.modifiers),
                WidgetKey::from_key_code(key),
            );
            route_outcome.merge(outcome);
        }
        if !repeat_accepted {
            return;
        }
        if !route_outcome.routed
            && !self.core.has_focused_text_input()
            && let Some(press) = logical_shortcut_keypress_from_text(logical_text)
        {
            let outcome = self.core.route_key_press(press, None);
            route_outcome.merge(outcome);
            if route_outcome.routed {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
        }
        if let Some(text) = event.text.as_ref() {
            self.route_text_input_after_unhandled_keypress(text, &mut route_outcome);
        } else if matches!(event.logical_key, Key::Named(NamedKey::Space)) {
            self.route_text_input_after_unhandled_keypress(" ", &mut route_outcome);
        } else if let Key::Character(text) = &event.logical_key {
            self.route_text_input_after_unhandled_keypress(text.as_str(), &mut route_outcome);
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_widget_key(WidgetKey::Backspace);
            route_outcome.merge(outcome);
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_widget_key(WidgetKey::Delete);
            route_outcome.merge(outcome);
        }
        self.handle_route_outcome(event_loop, route_outcome);
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
