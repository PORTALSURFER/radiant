use super::super::word_boundary::{next_word_boundary, previous_word_boundary};
use super::super::{TextInputEditResult, TextInputState};
use crate::widgets::primitives::text_input::editing_ops::{
    byte_index_for_char, sanitize_single_line_text,
};

impl TextInputState {
    /// Replace the active selection with sanitized single-line text.
    ///
    /// If no selection is active, this is a no-op and returns the default edit
    /// result. Use [`Self::insert_text`] when text should be inserted at the
    /// caret even without a selection.
    pub fn replace_selection(
        &mut self,
        replacement: &str,
        character_limit: Option<usize>,
    ) -> TextInputEditResult {
        if !self.has_selection() {
            return TextInputEditResult::default();
        }
        self.insert_text(replacement, character_limit)
    }

    /// Delete the active selection, if any.
    pub fn delete_selection(&mut self) -> TextInputEditResult {
        self.delete_selected_text()
    }

    /// Insert text at the current selection, sanitizing it to single-line input.
    pub fn insert_text(
        &mut self,
        text: &str,
        character_limit: Option<usize>,
    ) -> TextInputEditResult {
        let text = sanitize_single_line_text(text);
        if text.is_empty() {
            return TextInputEditResult::default();
        }
        let (selection_start, selection_end) = self.selection_range();
        let current_chars = self.char_len();
        let selected_chars = selection_end.saturating_sub(selection_start);
        let available = character_limit
            .map(|limit| limit.saturating_sub(current_chars.saturating_sub(selected_chars)));
        let insert_text = if let Some(available) = available {
            text.chars().take(available).collect::<String>()
        } else {
            text
        };
        if insert_text.is_empty() {
            return TextInputEditResult::default();
        }
        let start = byte_index_for_char(&self.value, selection_start);
        let end = byte_index_for_char(&self.value, selection_end);
        self.value.replace_range(start..end, &insert_text);
        self.caret = selection_start + insert_text.chars().count();
        self.selection_anchor = self.caret;
        TextInputEditResult {
            value_changed: true,
            selection_changed: true,
        }
    }

    pub(crate) fn backspace(&mut self) -> TextInputEditResult {
        if self.has_selection() {
            return self.delete_selected_text();
        }
        if self.caret == 0 {
            return TextInputEditResult::default();
        }
        let end = byte_index_for_char(&self.value, self.caret);
        let start = byte_index_for_char(&self.value, self.caret - 1);
        self.value.replace_range(start..end, "");
        self.caret -= 1;
        self.selection_anchor = self.caret;
        TextInputEditResult {
            value_changed: true,
            selection_changed: true,
        }
    }

    pub(crate) fn delete_forward(&mut self) -> TextInputEditResult {
        if self.has_selection() {
            return self.delete_selected_text();
        }
        if self.caret >= self.char_len() {
            return TextInputEditResult::default();
        }
        let start = byte_index_for_char(&self.value, self.caret);
        let end = byte_index_for_char(&self.value, self.caret + 1);
        self.value.replace_range(start..end, "");
        self.selection_anchor = self.caret;
        TextInputEditResult {
            value_changed: true,
            selection_changed: true,
        }
    }

    pub(crate) fn delete_word_left(&mut self) -> TextInputEditResult {
        if self.has_selection() {
            return self.delete_selected_text();
        }
        let target = previous_word_boundary(&self.value, self.caret);
        self.delete_char_range(target, self.caret)
    }

    pub(crate) fn delete_word_right(&mut self) -> TextInputEditResult {
        if self.has_selection() {
            return self.delete_selected_text();
        }
        let target = next_word_boundary(&self.value, self.caret);
        self.delete_char_range(self.caret, target)
    }

    pub(crate) fn delete_selected_text(&mut self) -> TextInputEditResult {
        let (selection_start, selection_end) = self.selection_range();
        self.delete_char_range(selection_start, selection_end)
    }

    fn delete_char_range(
        &mut self,
        selection_start: usize,
        selection_end: usize,
    ) -> TextInputEditResult {
        if selection_start == selection_end {
            return TextInputEditResult::default();
        }
        let start = byte_index_for_char(&self.value, selection_start);
        let end = byte_index_for_char(&self.value, selection_end);
        self.value.replace_range(start..end, "");
        self.caret = selection_start;
        self.selection_anchor = selection_start;
        TextInputEditResult {
            value_changed: true,
            selection_changed: true,
        }
    }
}
