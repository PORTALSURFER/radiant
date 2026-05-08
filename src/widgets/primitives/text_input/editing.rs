use crate::gui::types::Rect;
use crate::widgets::interaction::{TextEditCommand, TextInputMessage, WidgetKey};

use super::TextInputWidget;

impl TextInputWidget {
    /// Return the current selected text if the field has an active selection.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range();
        (start < end).then(|| {
            let start = byte_index_for_char(&self.state.value, start);
            let end = byte_index_for_char(&self.state.value, end);
            self.state.value[start..end].to_string()
        })
    }

    /// Return the selected character range sorted from start to end.
    pub fn selection_range(&self) -> (usize, usize) {
        if !self.has_selection() {
            return (self.state.caret, self.state.caret);
        }
        let start = self.state.selection_anchor.min(self.state.caret);
        let end = self
            .state
            .selection_anchor
            .max(self.state.caret)
            .saturating_add(1)
            .min(self.char_len());
        (start, end)
    }

    pub(super) fn handle_key_input(&mut self, key: WidgetKey) -> Option<TextInputMessage> {
        match key {
            WidgetKey::ArrowLeft => {
                self.move_left(false);
                None
            }
            WidgetKey::ArrowRight => {
                self.move_right(false);
                None
            }
            WidgetKey::Home => {
                self.move_to_start(false);
                None
            }
            WidgetKey::End => {
                self.move_to_end(false);
                None
            }
            WidgetKey::Backspace => self.backspace(),
            WidgetKey::Delete => self.delete_forward(),
            WidgetKey::Enter if self.props.submit_on_enter => Some(TextInputMessage::Submitted {
                value: self.state.value.clone(),
            }),
            _ => None,
        }
    }

    pub(super) fn handle_text_edit(
        &mut self,
        command: TextEditCommand,
    ) -> Option<TextInputMessage> {
        match command {
            TextEditCommand::MoveLeft { extend_selection } => {
                self.move_left(extend_selection);
                None
            }
            TextEditCommand::MoveRight { extend_selection } => {
                self.move_right(extend_selection);
                None
            }
            TextEditCommand::MoveHome { extend_selection } => {
                self.move_to_start(extend_selection);
                None
            }
            TextEditCommand::MoveEnd { extend_selection } => {
                self.move_to_end(extend_selection);
                None
            }
            TextEditCommand::SelectAll => {
                self.state.selection_anchor = 0;
                self.state.caret = self.char_len();
                None
            }
            TextEditCommand::InsertText(text) => {
                self.insert_text(&sanitize_single_line_text(&text))
            }
            TextEditCommand::Backspace => self.backspace(),
            TextEditCommand::Delete => self.delete_forward(),
            TextEditCommand::CutSelection => self.delete_selection(),
        }
    }

    pub(super) fn insert_text(&mut self, text: &str) -> Option<TextInputMessage> {
        if text.is_empty() {
            return None;
        }
        let (selection_start, selection_end) = self.selection_range();
        let current_chars = self.char_len();
        let selected_chars = selection_end.saturating_sub(selection_start);
        let available = self
            .props
            .character_limit
            .map(|limit| limit.saturating_sub(current_chars.saturating_sub(selected_chars)));
        let insert_text = if let Some(available) = available {
            text.chars().take(available).collect::<String>()
        } else {
            text.to_string()
        };
        if insert_text.is_empty() {
            return None;
        }
        let start = byte_index_for_char(&self.state.value, selection_start);
        let end = byte_index_for_char(&self.state.value, selection_end);
        self.state.value.replace_range(start..end, &insert_text);
        self.state.caret = selection_start + insert_text.chars().count();
        self.state.selection_anchor = self.state.caret;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn backspace(&mut self) -> Option<TextInputMessage> {
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.state.caret == 0 {
            return None;
        }
        let end = byte_index_for_char(&self.state.value, self.state.caret);
        let start = byte_index_for_char(&self.state.value, self.state.caret - 1);
        self.state.value.replace_range(start..end, "");
        self.state.caret -= 1;
        self.state.selection_anchor = self.state.caret;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn delete_forward(&mut self) -> Option<TextInputMessage> {
        if self.has_selection() {
            return self.delete_selection();
        }
        if self.state.caret >= self.char_len() {
            return None;
        }
        let start = byte_index_for_char(&self.state.value, self.state.caret);
        let end = byte_index_for_char(&self.state.value, self.state.caret + 1);
        self.state.value.replace_range(start..end, "");
        self.state.selection_anchor = self.state.caret;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn delete_selection(&mut self) -> Option<TextInputMessage> {
        let (selection_start, selection_end) = self.selection_range();
        if selection_start == selection_end {
            return None;
        }
        let start = byte_index_for_char(&self.state.value, selection_start);
        let end = byte_index_for_char(&self.state.value, selection_end);
        self.state.value.replace_range(start..end, "");
        self.state.caret = selection_start;
        self.state.selection_anchor = selection_start;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn move_left(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            self.state.caret.saturating_sub(1)
        };
        self.set_caret(target, extend_selection);
    }

    fn move_right(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            (self.state.caret + 1).min(self.char_len())
        };
        self.set_caret(target, extend_selection);
    }

    fn move_to_start(&mut self, extend_selection: bool) {
        self.set_caret(0, extend_selection);
    }

    fn move_to_end(&mut self, extend_selection: bool) {
        self.set_caret(self.char_len(), extend_selection);
    }

    pub(super) fn set_caret(&mut self, caret: usize, extend_selection: bool) {
        self.state.caret = caret.min(self.char_len());
        if !extend_selection {
            self.state.selection_anchor = self.state.caret;
        }
    }

    fn has_selection(&self) -> bool {
        self.state.selection_anchor != self.state.caret
    }

    fn char_len(&self) -> usize {
        self.state.value.chars().count()
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

pub(super) fn caret_for_pointer_x(bounds: Rect, x: f32) -> usize {
    let text_x = (x - bounds.min.x - 16.0).max(0.0);
    let font_size: f32 = if bounds.height() >= 42.0 { 15.0 } else { 13.0 };
    let char_width = (font_size * 0.58_f32).max(1.0_f32);
    (text_x / char_width).floor().max(0.0) as usize
}

fn sanitize_single_line_text(text: &str) -> String {
    let mut sanitized = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\r' | '\n' => {}
            '\t' => sanitized.push(' '),
            _ if ch.is_control() => {}
            _ => sanitized.push(ch),
        }
    }
    sanitized
}
