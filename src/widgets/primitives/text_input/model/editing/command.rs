use super::super::{TextInputEditResult, TextInputState};
use crate::widgets::{
    interaction::{TextEditCommand, WidgetKey},
    primitives::text_input::editing_ops::sanitize_single_line_text,
};

impl TextInputState {
    /// Apply a high-level text-edit command and report whether it changed state.
    pub fn apply_edit_command(
        &mut self,
        command: TextEditCommand,
        character_limit: Option<usize>,
    ) -> TextInputEditResult {
        match command {
            TextEditCommand::MoveLeft { extend_selection } => {
                self.move_left(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::MoveRight { extend_selection } => {
                self.move_right(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::MoveWordLeft { extend_selection } => {
                self.move_word_left(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::MoveWordRight { extend_selection } => {
                self.move_word_right(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::MoveHome { extend_selection } => {
                self.move_to_start(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::MoveEnd { extend_selection } => {
                self.move_to_end(extend_selection);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::SelectAll => {
                self.selection_anchor = 0;
                self.caret = self.char_len();
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            TextEditCommand::InsertText(text) => {
                self.insert_text(&sanitize_single_line_text(&text), character_limit)
            }
            TextEditCommand::Backspace => self.backspace(),
            TextEditCommand::Delete => self.delete_forward(),
            TextEditCommand::DeleteWordLeft => self.delete_word_left(),
            TextEditCommand::DeleteWordRight => self.delete_word_right(),
            TextEditCommand::CutSelection => self.delete_selected_text(),
        }
    }

    /// Apply a portable key command that has text-editing semantics.
    pub fn apply_key(&mut self, key: WidgetKey) -> TextInputEditResult {
        match key {
            WidgetKey::ArrowLeft => {
                self.move_left(false);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            WidgetKey::ArrowRight => {
                self.move_right(false);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            WidgetKey::Home => {
                self.move_to_start(false);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            WidgetKey::End => {
                self.move_to_end(false);
                TextInputEditResult {
                    selection_changed: true,
                    ..TextInputEditResult::default()
                }
            }
            WidgetKey::Backspace => self.backspace(),
            WidgetKey::Delete => self.delete_forward(),
            WidgetKey::Enter | WidgetKey::Space | WidgetKey::ArrowUp | WidgetKey::ArrowDown => {
                TextInputEditResult::default()
            }
        }
    }
}
