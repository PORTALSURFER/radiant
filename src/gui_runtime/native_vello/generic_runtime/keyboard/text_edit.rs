use super::keypress_from_input;
use super::{GenericNativeVelloRunner, GenericRouteOutcome};
use crate::gui::input::InputTimestamp;
use crate::gui::input::KeyCode;
use crate::runtime::RuntimeBridge;
use crate::widgets::KeyboardModifiers;
use crate::widgets::TextEditCommand;
use crate::widgets::WidgetKey;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn route_space_text_input(
        &mut self,
        key: KeyCode,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if key != KeyCode::Space
            || self.input.modifiers.control_key()
            || self.input.modifiers.super_key()
            || self.input.modifiers.alt_key()
            || !self.core.has_focused_text_input()
        {
            return false;
        }
        self.route_text_input(" ", timestamp, route_outcome);
        route_outcome.routed
    }

    pub(super) fn route_text_input_shortcut(
        &mut self,
        key: KeyCode,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if !(self.input.modifiers.control_key() || self.input.modifiers.super_key()) {
            return false;
        }
        match key {
            KeyCode::A => {
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::SelectAll, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::C => {
                if let Some(selection) = self.core.focused_text_selection() {
                    if let Some(clipboard) = &mut self.input.clipboard {
                        let _ = clipboard.set_text(selection);
                    }
                    route_outcome.routed = true;
                    return true;
                }
                false
            }
            KeyCode::X => {
                if let Some(selection) = self.core.focused_text_selection() {
                    if let Some(clipboard) = &mut self.input.clipboard {
                        let _ = clipboard.set_text(selection);
                    }
                    let outcome = self
                        .core
                        .route_text_edit_with_timestamp(TextEditCommand::CutSelection, timestamp);
                    route_outcome.merge(outcome);
                    return outcome.routed;
                }
                false
            }
            KeyCode::V => {
                let Some(clipboard) = &mut self.input.clipboard else {
                    return false;
                };
                let Ok(text) = clipboard.get_text() else {
                    return false;
                };
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::InsertText(text), timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Backspace => {
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::DeleteWordLeft, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Delete => {
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::DeleteWordRight, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            _ => false,
        }
    }

    pub(super) fn route_focused_widget_preempting_shortcut_key(
        &mut self,
        key: KeyCode,
        timestamp: Option<InputTimestamp>,
        repeat: bool,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if self.input.modifiers.control_key()
            || self.input.modifiers.super_key()
            || self.input.modifiers.alt_key()
        {
            return false;
        }
        let Some(widget_key) = WidgetKey::from_key_code(key) else {
            return false;
        };
        if !self
            .core
            .focused_widget_preempts_host_shortcut_key(widget_key)
        {
            return false;
        }
        let press = keypress_from_input(key, self.input.modifiers);
        let outcome = self.core.route_widget_key_with_metadata(
            widget_key,
            KeyboardModifiers::from(press),
            repeat,
            timestamp,
        );
        route_outcome.merge(outcome);
        outcome.routed
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn route_focused_text_input_before_shortcuts(
        &mut self,
        key: KeyCode,
        text: Option<&str>,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if self.input.modifiers.control_key()
            || self.input.modifiers.super_key()
            || self.input.modifiers.alt_key()
            || !self.core.has_focused_text_input()
        {
            return false;
        }

        match key {
            KeyCode::Backspace => {
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::Backspace, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Delete => {
                let outcome = self
                    .core
                    .route_text_edit_with_timestamp(TextEditCommand::Delete, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Enter => {
                let outcome = self
                    .core
                    .route_widget_key_with_timestamp(WidgetKey::Enter, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Tab => {
                let outcome = self
                    .core
                    .route_widget_key_with_timestamp(WidgetKey::Tab, timestamp);
                route_outcome.merge(outcome);
                outcome.routed
            }
            _ => {
                let Some(text) = text else {
                    return false;
                };
                self.route_text_input(text, timestamp, route_outcome);
                route_outcome.routed
            }
        }
    }

    pub(super) fn route_text_navigation_key(
        &mut self,
        key: KeyCode,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        let extend_selection = self.input.modifiers.shift_key();
        let word_navigation =
            self.input.modifiers.control_key() || self.input.modifiers.super_key();
        let command = match key {
            KeyCode::ArrowLeft if word_navigation => {
                TextEditCommand::MoveWordLeft { extend_selection }
            }
            KeyCode::ArrowRight if word_navigation => {
                TextEditCommand::MoveWordRight { extend_selection }
            }
            KeyCode::ArrowLeft => TextEditCommand::MoveLeft { extend_selection },
            KeyCode::ArrowRight => TextEditCommand::MoveRight { extend_selection },
            KeyCode::Home => TextEditCommand::MoveHome { extend_selection },
            KeyCode::End => TextEditCommand::MoveEnd { extend_selection },
            _ => return false,
        };
        let outcome = self.core.route_text_edit_with_timestamp(command, timestamp);
        route_outcome.merge(outcome);
        outcome.routed
    }

    /// Route printable text from a keyboard event into the focused widget.
    pub(super) fn route_text_input(
        &mut self,
        text: &str,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) {
        for character in text.chars().filter(|character| !character.is_control()) {
            let outcome = self
                .core
                .route_character_with_timestamp(character, timestamp);
            route_outcome.merge(outcome);
        }
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime) fn route_text_input_after_unhandled_keypress(
        &mut self,
        text: &str,
        timestamp: Option<InputTimestamp>,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if route_outcome.routed {
            return false;
        }
        self.route_text_input(text, timestamp, route_outcome);
        route_outcome.routed
    }
}
