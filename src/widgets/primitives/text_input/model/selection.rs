use super::TextInputState;
use super::word_boundary::word_range_at;
use crate::widgets::primitives::text_input::editing_ops::byte_index_for_char;

impl TextInputState {
    /// Return the current selected text as an owned string.
    pub fn selected_text(&self) -> Option<String> {
        self.selected_text_slice().map(str::to_owned)
    }

    /// Return the current selected text as a borrowed UTF-8 slice.
    pub fn selected_text_slice(&self) -> Option<&str> {
        let (start, end) = self.selection_range();
        (start < end).then(|| {
            let start = byte_index_for_char(&self.value, start);
            let end = byte_index_for_char(&self.value, end);
            &self.value[start..end]
        })
    }

    /// Return the selected character range sorted from start to end.
    pub fn selection_range(&self) -> (usize, usize) {
        let char_len = self.char_len();
        let caret = self.caret.min(char_len);
        let anchor = self.selection_anchor.min(char_len);
        if anchor == caret {
            return (caret, caret);
        }
        let start = anchor.min(caret);
        let end = anchor.max(caret).saturating_add(1).min(char_len);
        (start, end)
    }

    /// Return whether the state currently has an active non-empty selection.
    pub fn has_selection(&self) -> bool {
        let (start, end) = self.selection_range();
        start < end
    }

    /// Collapse the current selection at the caret.
    pub fn clear_selection(&mut self) {
        self.selection_anchor = self.caret.min(self.char_len());
    }

    /// Select the word at or immediately before a character index.
    ///
    /// Returns false when the requested index is not adjacent to word text.
    pub fn select_word_at(&mut self, caret: usize) -> bool {
        let Some((start, end)) = word_range_at(&self.value, caret) else {
            self.set_caret(caret, false);
            return false;
        };
        self.selection_anchor = start;
        self.caret = end.saturating_sub(1);
        true
    }
}
