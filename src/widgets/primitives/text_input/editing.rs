use crate::widgets::interaction::{TextEditCommand, TextInputMessage, WidgetKey};

use super::TextInputWidget;

impl TextInputWidget {
    /// Return the current selected text if the field has an active selection.
    pub fn selected_text(&self) -> Option<String> {
        self.state.selected_text()
    }

    /// Return the selected character range sorted from start to end.
    pub fn selection_range(&self) -> (usize, usize) {
        self.state.selection_range()
    }

    pub(super) fn handle_key_input(&mut self, key: WidgetKey) -> Option<TextInputMessage> {
        match key {
            WidgetKey::Enter if self.props.submit_on_enter => Some(TextInputMessage::Submitted {
                value: self.state.value.clone(),
            }),
            _ => {
                let result = self.state.apply_key(key);
                result.value_changed.then(|| TextInputMessage::Changed {
                    value: self.state.value.clone(),
                })
            }
        }
    }

    pub(super) fn handle_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> Option<TextInputMessage> {
        let result = self
            .state
            .apply_edit_command(command, self.props.character_limit);
        result.value_changed.then(|| TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    pub(super) fn insert_text(&mut self, text: &str) -> Option<TextInputMessage> {
        let result = self.state.insert_text(text, self.props.character_limit);
        result.value_changed.then(|| TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    pub(super) fn set_caret(&mut self, caret: usize, extend_selection: bool) {
        self.state.set_caret(caret, extend_selection);
    }
}
