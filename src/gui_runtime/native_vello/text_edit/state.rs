/// Mutable selection/caret state for one single-line text field.
use super::boundary::clamp_to_char_boundary;

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
}
