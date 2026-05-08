use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_keyboard_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: winit::event::KeyEvent,
    ) {
        if event.state != ElementState::Pressed || event.repeat {
            return;
        }
        let mut route_outcome = GenericRouteOutcome::default();
        if let PhysicalKey::Code(code) = event.physical_key
            && let Some(key) = key_code_from_winit(code)
        {
            if self.route_text_input_shortcut(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_text_navigation_key(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            if self.route_space_text_input(key, &mut route_outcome) {
                self.handle_route_outcome(event_loop, route_outcome);
                return;
            }
            let outcome = self.core.route_key_press(
                keypress_from_input(key, self.modifiers),
                WidgetKey::from_key_code(key),
            );
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if let Some(text) = event.text.as_ref() {
            self.route_text_input(text, &mut route_outcome);
        } else if matches!(event.logical_key, Key::Named(NamedKey::Space)) {
            self.route_text_input(" ", &mut route_outcome);
        } else if let Key::Character(text) = &event.logical_key {
            self.route_text_input(text.as_str(), &mut route_outcome);
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Backspace)) {
            let outcome = self.core.route_widget_key(WidgetKey::Backspace);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        if !route_outcome.routed && matches!(event.logical_key, Key::Named(NamedKey::Delete)) {
            let outcome = self.core.route_widget_key(WidgetKey::Delete);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
        self.handle_route_outcome(event_loop, route_outcome);
    }

    fn route_space_text_input(
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

    fn route_text_input_shortcut(
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
                route_outcome.routed |= outcome.routed;
                route_outcome.repaint_requested |= outcome.repaint_requested;
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
                    route_outcome.routed |= outcome.routed;
                    route_outcome.repaint_requested |= outcome.repaint_requested;
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
                route_outcome.routed |= outcome.routed;
                route_outcome.repaint_requested |= outcome.repaint_requested;
                outcome.routed
            }
            _ => false,
        }
    }

    fn route_text_navigation_key(
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
        route_outcome.routed |= outcome.routed;
        route_outcome.repaint_requested |= outcome.repaint_requested;
        outcome.routed
    }

    /// Route printable text from a keyboard event into the focused widget.
    fn route_text_input(&mut self, text: &str, route_outcome: &mut GenericRouteOutcome) {
        for character in text.chars().filter(|character| !character.is_control()) {
            if route_outcome.routed {
                break;
            }
            let outcome = self.core.route_character(character);
            route_outcome.routed |= outcome.routed;
            route_outcome.repaint_requested |= outcome.repaint_requested;
        }
    }
}
