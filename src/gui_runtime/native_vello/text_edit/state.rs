/// Mutable selection/caret state for one single-line text field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct SingleLineTextEditorState {
    pub(in crate::gui_runtime::native_vello) anchor_byte: usize,
    pub(in crate::gui_runtime::native_vello) cursor_byte: usize,
    pub(in crate::gui_runtime::native_vello) scroll_start_byte: usize,
}

impl SingleLineTextEditorState {
    /// Create a collapsed editor state at the end of `text`.
    pub(in crate::gui_runtime::native_vello) fn collapsed_at_end(text: &str) -> Self {
        let byte = text.len();
        Self {
            anchor_byte: byte,
            cursor_byte: byte,
            scroll_start_byte: 0,
        }
    }

    /// Clamp caret, selection, and scroll state to valid UTF-8 boundaries.
    pub(in crate::gui_runtime::native_vello) fn clamp_to_text(&mut self, text: &str) {
        self.anchor_byte = clamp_to_char_boundary(text, self.anchor_byte);
        self.cursor_byte = clamp_to_char_boundary(text, self.cursor_byte);
        self.scroll_start_byte = clamp_to_char_boundary(text, self.scroll_start_byte);
    }

    /// Return the sorted active selection range in bytes.
    pub(in crate::gui_runtime::native_vello) fn selection_range(&self) -> (usize, usize) {
        (
            self.anchor_byte.min(self.cursor_byte),
            self.anchor_byte.max(self.cursor_byte),
        )
    }

    /// Return whether the editor currently has a non-empty selection.
    pub(in crate::gui_runtime::native_vello) fn has_selection(&self) -> bool {
        self.anchor_byte != self.cursor_byte
    }

    /// Move the caret/selection to `byte_index`.
    pub(in crate::gui_runtime::native_vello) fn set_cursor(
        &mut self,
        text: &str,
        byte_index: usize,
        extend_selection: bool,
    ) {
        let clamped = clamp_to_char_boundary(text, byte_index);
        if extend_selection {
            self.cursor_byte = clamped;
        } else {
            self.anchor_byte = clamped;
            self.cursor_byte = clamped;
        }
    }

    /// Select the entire text payload.
    pub(in crate::gui_runtime::native_vello) fn select_all(&mut self, text: &str) {
        self.anchor_byte = 0;
        self.cursor_byte = text.len();
        self.scroll_start_byte = 0;
    }

    /// Move the caret one character to the left.
    pub(in crate::gui_runtime::native_vello) fn move_left(
        &mut self,
        text: &str,
        extend_selection: bool,
    ) -> bool {
        self.clamp_to_text(text);
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            previous_char_boundary(text, self.cursor_byte)
        };
        if target == self.cursor_byte && (!extend_selection || self.anchor_byte == target) {
            return false;
        }
        self.set_cursor(text, target, extend_selection);
        true
    }

    /// Move the caret one character to the right.
    pub(in crate::gui_runtime::native_vello) fn move_right(
        &mut self,
        text: &str,
        extend_selection: bool,
    ) -> bool {
        self.clamp_to_text(text);
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            next_char_boundary(text, self.cursor_byte)
        };
        if target == self.cursor_byte && (!extend_selection || self.anchor_byte == target) {
            return false;
        }
        self.set_cursor(text, target, extend_selection);
        true
    }

    /// Move the caret to the start of the text.
    pub(in crate::gui_runtime::native_vello) fn move_home(
        &mut self,
        text: &str,
        extend_selection: bool,
    ) -> bool {
        self.clamp_to_text(text);
        if self.cursor_byte == 0 && (!extend_selection || self.anchor_byte == 0) {
            return false;
        }
        self.set_cursor(text, 0, extend_selection);
        true
    }

    /// Move the caret to the end of the text.
    pub(in crate::gui_runtime::native_vello) fn move_end(
        &mut self,
        text: &str,
        extend_selection: bool,
    ) -> bool {
        self.clamp_to_text(text);
        let end = text.len();
        if self.cursor_byte == end && (!extend_selection || self.anchor_byte == end) {
            return false;
        }
        self.set_cursor(text, end, extend_selection);
        true
    }

    /// Replace the selected range with `replacement` and collapse after it.
    pub(in crate::gui_runtime::native_vello) fn replace_selection(
        &mut self,
        text: &str,
        replacement: &str,
    ) -> String {
        self.clamp_to_text(text);
        let (start, end) = self.selection_range();
        let mut next =
            String::with_capacity(text.len() + replacement.len().saturating_sub(end - start));
        next.push_str(&text[..start]);
        next.push_str(replacement);
        next.push_str(&text[end..]);
        let caret = start + replacement.len();
        self.anchor_byte = caret;
        self.cursor_byte = caret;
        self.scroll_start_byte = self.scroll_start_byte.min(caret);
        next
    }

    /// Delete the current selection or the previous character.
    pub(in crate::gui_runtime::native_vello) fn backspace(&mut self, text: &str) -> Option<String> {
        self.clamp_to_text(text);
        if self.has_selection() {
            return Some(self.replace_selection(text, ""));
        }
        let previous = previous_char_boundary(text, self.cursor_byte);
        if previous == self.cursor_byte {
            return None;
        }
        self.anchor_byte = previous;
        Some(self.replace_selection(text, ""))
    }

    /// Delete the current selection or the next character.
    pub(in crate::gui_runtime::native_vello) fn delete_forward(
        &mut self,
        text: &str,
    ) -> Option<String> {
        self.clamp_to_text(text);
        if self.has_selection() {
            return Some(self.replace_selection(text, ""));
        }
        let next = next_char_boundary(text, self.cursor_byte);
        if next == self.cursor_byte {
            return None;
        }
        self.anchor_byte = self.cursor_byte;
        self.cursor_byte = next;
        Some(self.replace_selection(text, ""))
    }

    /// Return the currently selected text, if any.
    pub(in crate::gui_runtime::native_vello) fn selected_text(&self, text: &str) -> Option<String> {
        let (start, end) = self.clamped_selection_range(text);
        (start < end).then(|| text[start..end].to_string())
    }

    fn clamped_selection_range(&self, text: &str) -> (usize, usize) {
        let start = clamp_to_char_boundary(text, self.anchor_byte.min(self.cursor_byte));
        let end = clamp_to_char_boundary(text, self.anchor_byte.max(self.cursor_byte));
        (start, end)
    }
}

fn clamp_to_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = byte_index.min(text.len());
    if text.is_char_boundary(clamped) {
        return clamped;
    }
    let mut last = 0;
    for (idx, _) in text.char_indices() {
        if idx >= clamped {
            break;
        }
        last = idx;
    }
    last
}

fn previous_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    text[..clamped]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, byte_index: usize) -> usize {
    let clamped = clamp_to_char_boundary(text, byte_index);
    if clamped >= text.len() {
        return text.len();
    }
    let mut iter = text[clamped..].char_indices();
    let _ = iter.next();
    iter.next()
        .map(|(idx, _)| clamped + idx)
        .unwrap_or(text.len())
}
