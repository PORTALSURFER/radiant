use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn route_space_text_input(
        &mut self,
        key: crate::gui::input::KeyCode,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if key != crate::gui::input::KeyCode::Space
            || self.modifiers.control_key()
            || self.modifiers.super_key()
            || self.modifiers.alt_key()
            || !self.core.has_focused_text_input()
        {
            return false;
        }
        self.route_text_input(" ", route_outcome);
        route_outcome.routed
    }

    pub(super) fn route_text_input_shortcut(
        &mut self,
        key: crate::gui::input::KeyCode,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        if !(self.modifiers.control_key() || self.modifiers.super_key()) {
            return false;
        }
        match key {
            crate::gui::input::KeyCode::A => {
                let outcome = self.core.route_text_edit(TextEditCommand::SelectAll);
                route_outcome.merge(outcome);
                outcome.routed
            }
            crate::gui::input::KeyCode::C => {
                if let Some(selection) = self.core.focused_text_selection() {
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(selection);
                    }
                    route_outcome.routed = true;
                    return true;
                }
                false
            }
            crate::gui::input::KeyCode::X => {
                if let Some(selection) = self.core.focused_text_selection() {
                    if let Some(clipboard) = &mut self.clipboard {
                        let _ = clipboard.set_text(selection);
                    }
                    let outcome = self.core.route_text_edit(TextEditCommand::CutSelection);
                    route_outcome.merge(outcome);
                    return outcome.routed;
                }
                false
            }
            crate::gui::input::KeyCode::V => {
                let Some(clipboard) = &mut self.clipboard else {
                    return false;
                };
                let Ok(text) = clipboard.get_text() else {
                    return false;
                };
                let outcome = self.core.route_text_edit(TextEditCommand::InsertText(text));
                route_outcome.merge(outcome);
                outcome.routed
            }
            _ => false,
        }
    }

    pub(super) fn route_text_navigation_key(
        &mut self,
        key: crate::gui::input::KeyCode,
        route_outcome: &mut GenericRouteOutcome,
    ) -> bool {
        let extend_selection = self.modifiers.shift_key();
        let command = match key {
            crate::gui::input::KeyCode::ArrowLeft => TextEditCommand::MoveLeft { extend_selection },
            crate::gui::input::KeyCode::ArrowRight => {
                TextEditCommand::MoveRight { extend_selection }
            }
            crate::gui::input::KeyCode::Home => TextEditCommand::MoveHome { extend_selection },
            crate::gui::input::KeyCode::End => TextEditCommand::MoveEnd { extend_selection },
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
