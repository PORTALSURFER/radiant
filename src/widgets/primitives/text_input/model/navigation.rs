use super::TextInputState;
use super::word_boundary::{next_word_boundary, previous_word_boundary};

impl TextInputState {
    /// Move the caret to a character index, optionally extending selection.
    pub fn set_caret(&mut self, caret: usize, extend_selection: bool) {
        self.caret = caret.min(self.char_len());
        if !extend_selection {
            self.selection_anchor = self.caret;
        }
    }

    pub(super) fn move_left(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            self.caret.saturating_sub(1)
        };
        self.set_caret(target, extend_selection);
    }

    pub(super) fn move_right(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            (self.caret + 1).min(self.char_len())
        };
        self.set_caret(target, extend_selection);
    }

    pub(super) fn move_word_left(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            previous_word_boundary(&self.value, self.caret)
        };
        self.set_caret(target, extend_selection);
    }

    pub(super) fn move_word_right(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            let boundary = next_word_boundary(&self.value, self.caret);
            if extend_selection && boundary > 0 {
                boundary - 1
            } else {
                boundary
            }
        };
        self.set_caret(target, extend_selection);
    }

    pub(super) fn move_to_start(&mut self, extend_selection: bool) {
        self.set_caret(0, extend_selection);
    }

    pub(super) fn move_to_end(&mut self, extend_selection: bool) {
        self.set_caret(self.char_len(), extend_selection);
    }
}
