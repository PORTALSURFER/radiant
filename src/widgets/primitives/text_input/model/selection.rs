use super::TextInputState;
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
}
