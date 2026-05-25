use super::{GenericNativeVelloRunner, GenericRouteOutcome};
use crate::gui::input::KeyCode;
use crate::runtime::RuntimeBridge;
use crate::widgets::TextEditCommand;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn route_space_text_input(
        &mut self,
        key: KeyCode,
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
        self.route_text_input(" ", route_outcome);
        route_outcome.routed
    }

    pub(super) fn route_text_input_shortcut(
        &mut self,
        key: KeyCode,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if !(self.input.modifiers.control_key() || self.input.modifiers.super_key()) {
            return false;
        }
        match key {
            KeyCode::A => {
                let outcome = self.core.route_text_edit(TextEditCommand::SelectAll);
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
                    let outcome = self.core.route_text_edit(TextEditCommand::CutSelection);
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
                let outcome = self.core.route_text_edit(TextEditCommand::InsertText(text));
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Backspace => {
                let outcome = self.core.route_text_edit(TextEditCommand::DeleteWordLeft);
                route_outcome.merge(outcome);
                outcome.routed
            }
            KeyCode::Delete => {
                let outcome = self.core.route_text_edit(TextEditCommand::DeleteWordRight);
                route_outcome.merge(outcome);
                outcome.routed
            }
            _ => false,
        }
    }

    pub(super) fn route_text_navigation_key(
        &mut self,
        key: KeyCode,
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
        let outcome = self.core.route_text_edit(command);
        route_outcome.merge(outcome);
        outcome.routed
    }

    /// Route printable text from a keyboard event into the focused widget.
    pub(super) fn route_text_input(&mut self, text: &str, route_outcome: &mut GenericRouteOutcome) {
        for character in text.chars().filter(|character| !character.is_control()) {
            if route_outcome.routed {
                break;
            }
            let outcome = self.core.route_character(character);
            route_outcome.merge(outcome);
        }
    }
}
