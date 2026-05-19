use super::TextInputState;

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

    pub(super) fn move_to_start(&mut self, extend_selection: bool) {
        self.set_caret(0, extend_selection);
    }

    pub(super) fn move_to_end(&mut self, extend_selection: bool) {
        self.set_caret(self.char_len(), extend_selection);
    }
}
