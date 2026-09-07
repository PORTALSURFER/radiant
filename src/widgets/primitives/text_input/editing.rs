use crate::widgets::interaction::{TextEditCommand, TextInputMessage, WidgetKey};

use super::TextInputWidget;

impl TextInputWidget {
    /// Return the current selected text if the field has an active selection.
    pub fn selected_text(&self) -> Option<String> {
        self.state.selected_text()
    }

    /// Return the current selected text as a borrowed UTF-8 slice.
    pub fn selected_text_slice(&self) -> Option<&str> {
        self.state.selected_text_slice()
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
            WidgetKey::Tab => Some(TextInputMessage::CompletionRequested {
                value: self.state.value.clone(),
            }),
            _ => {
                let result = self.state.apply_key(key);
                if result.value_changed || result.selection_changed {
                    self.invalidate_text_edit_authority();
                }
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
        if result.value_changed || result.selection_changed {
            self.invalidate_text_edit_authority();
        }
        result.value_changed.then(|| TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    pub(super) fn insert_text(&mut self, text: &str) -> Option<TextInputMessage> {
        let result = self.state.insert_text(text, self.props.character_limit);
        if result.value_changed || result.selection_changed {
            self.invalidate_text_edit_authority();
        }
        result.value_changed.then(|| TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    pub(super) fn set_caret(&mut self, caret: usize, extend_selection: bool) {
        let previous = (self.state.caret, self.state.selection_anchor);
        self.state.set_caret(caret, extend_selection);
        if previous != (self.state.caret, self.state.selection_anchor) {
            self.invalidate_text_edit_authority();
        }
    }

    pub(super) fn select_word_at(&mut self, caret: usize) -> bool {
        let previous = (self.state.caret, self.state.selection_anchor);
        let selected = self.state.select_word_at(caret);
        if previous != (self.state.caret, self.state.selection_anchor) {
            self.invalidate_text_edit_authority();
        }
        selected
    }
}
