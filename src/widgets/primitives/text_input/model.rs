use crate::runtime::PaintText;
use crate::widgets::interaction::{TextEditCommand, WidgetKey};

use super::editing_ops::{byte_index_for_char, sanitize_single_line_text};

/// Immutable public properties for a reusable single-line text input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextInputProps {
    /// Optional placeholder shown when the current value is empty.
    pub placeholder: Option<PaintText>,
    /// Whether Enter should emit a submit message instead of inserting text.
    pub submit_on_enter: bool,
    /// Optional maximum number of Unicode scalar values accepted by the field.
    pub character_limit: Option<usize>,
}

/// Mutable interaction state for a reusable single-line text input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextInputState {
    /// Current single-line text value.
    pub value: String,
    /// Caret position measured in Unicode scalar values from the start.
    pub caret: usize,
    /// Selection anchor measured in Unicode scalar values from the start.
    pub selection_anchor: usize,
}

/// Result of applying an editing command to [`TextInputState`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextInputEditResult {
    /// The text value changed and the host should publish a changed value.
    pub value_changed: bool,
    /// The caret or selection changed without necessarily changing the value.
    pub selection_changed: bool,
}

impl TextInputState {
    /// Create editable single-line text state with a collapsed caret at the end.
    pub fn from_value(value: String) -> Self {
        let caret = value.chars().count();
        Self {
            value,
            caret,
            selection_anchor: caret,
        }
    }

    /// Return the current selected text if the state has an active selection.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range();
        (start < end).then(|| {
            let start = byte_index_for_char(&self.value, start);
            let end = byte_index_for_char(&self.value, end);
            self.value[start..end].to_string()
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

    /// Move the caret to a character index, optionally extending selection.
    pub fn set_caret(&mut self, caret: usize, extend_selection: bool) {
        self.caret = caret.min(self.char_len());
        if !extend_selection {
            self.selection_anchor = self.caret;
        }
    }

    /// Return the current value length in Unicode scalar values.
    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    fn backspace(&mut self) -> TextInputEditResult {
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

    fn delete_forward(&mut self) -> TextInputEditResult {
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

    fn delete_selected_text(&mut self) -> TextInputEditResult {
        let (selection_start, selection_end) = self.selection_range();
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

    fn move_left(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().0
        } else {
            self.caret.saturating_sub(1)
        };
        self.set_caret(target, extend_selection);
    }

    fn move_right(&mut self, extend_selection: bool) {
        let target = if !extend_selection && self.has_selection() {
            self.selection_range().1
        } else {
            (self.caret + 1).min(self.char_len())
        };
        self.set_caret(target, extend_selection);
    }

    fn move_to_start(&mut self, extend_selection: bool) {
        self.set_caret(0, extend_selection);
    }

    fn move_to_end(&mut self, extend_selection: bool) {
        self.set_caret(self.char_len(), extend_selection);
    }
}
