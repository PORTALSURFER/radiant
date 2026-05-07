//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;

use super::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, WidgetId, WidgetKind, WidgetMessageKind, WidgetSizing,
};
use crate::widgets::interaction::{
    PointerButton, TextEditCommand, TextInputMessage, WidgetInput, WidgetKey,
};

/// Immutable public properties for a reusable single-line text input.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextInputProps {
    /// Optional placeholder shown when the current value is empty.
    pub placeholder: Option<String>,
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

impl TextInputState {
    fn new(value: String) -> Self {
        let caret = value.chars().count();
        Self {
            value,
            caret,
            selection_anchor: caret,
        }
    }
}

/// Public single-line text-input primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing text-input configuration.
    pub props: TextInputProps,
    /// Mutable input state owned by the widget.
    pub state: TextInputState,
}

impl TextInputWidget {
    /// Build a single-line text-input descriptor with edit semantics.
    pub fn new(id: WidgetId, value: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, WidgetKind::TextInput, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.emitted_messages.push(WidgetMessageKind::TextEdited);
        Self {
            common,
            props: TextInputProps {
                placeholder: None,
                submit_on_enter: true,
                character_limit: None,
            },
            state: TextInputState::new(value.into()),
        }
    }

    /// Route one backend-neutral interaction into the single-line text input.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<TextInputMessage> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.focused = true;
                self.common.state.hovered = true;
                self.move_to_end(false);
                None
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::Character(ch)
                if self.common.state.focused
                    && !self.common.state.disabled
                    && !self.common.state.read_only
                    && !ch.is_control() =>
            {
                self.insert_text(ch.encode_utf8(&mut [0; 4]))
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused
                    && !self.common.state.disabled
                    && !self.common.state.read_only =>
            {
                self.handle_key_input(key)
            }
            WidgetInput::TextEdit(command)
                if self.common.state.focused
                    && !self.common.state.disabled
                    && !self.common.state.read_only =>
            {
                self.handle_text_edit(command)
            }
            _ => None,
        }
    }

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
        (
            self.state.selection_anchor.min(self.state.caret),
            self.state.selection_anchor.max(self.state.caret),
        )
    }

    fn handle_key_input(&mut self, key: WidgetKey) -> Option<TextInputMessage> {
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

    fn handle_text_edit(&mut self, command: TextEditCommand) -> Option<TextInputMessage> {
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

    fn insert_text(&mut self, text: &str) -> Option<TextInputMessage> {
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

    fn set_caret(&mut self, caret: usize, extend_selection: bool) {
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

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;

    #[test]
    fn text_input_editing_emits_changed_and_submitted_messages() {
        let mut input = TextInputWidget::new(
            7,
            "ab",
            WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
        let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));
        input.state.caret = 1;
        input.state.selection_anchor = 1;

        assert_eq!(
            input.handle_input(bounds, WidgetInput::Character('z')),
            Some(TextInputMessage::Changed {
                value: String::from("azb"),
            })
        );
        assert_eq!(input.state.caret, 2);

        assert_eq!(
            input.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Backspace)),
            Some(TextInputMessage::Changed {
                value: String::from("ab"),
            })
        );

        assert_eq!(
            input.handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Enter)),
            Some(TextInputMessage::Submitted {
                value: String::from("ab"),
            })
        );
    }

    #[test]
    fn text_input_selection_replaces_cuts_and_pastes_text() {
        let mut input = TextInputWidget::new(
            7,
            "alpha beta",
            WidgetSizing::new(Vector2::new(100.0, 28.0), Vector2::new(160.0, 28.0)),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(160.0, 28.0));
        let _ = input.handle_input(bounds, WidgetInput::FocusChanged(true));

        let _ = input.handle_input(
            bounds,
            WidgetInput::TextEdit(TextEditCommand::MoveHome {
                extend_selection: false,
            }),
        );
        for _ in 0..5 {
            let _ = input.handle_input(
                bounds,
                WidgetInput::TextEdit(TextEditCommand::MoveRight {
                    extend_selection: true,
                }),
            );
        }

        assert_eq!(input.selected_text().as_deref(), Some("alpha"));
        assert_eq!(
            input.handle_input(
                bounds,
                WidgetInput::TextEdit(TextEditCommand::InsertText(String::from("one\ntwo"))),
            ),
            Some(TextInputMessage::Changed {
                value: String::from("onetwo beta"),
            })
        );

        let _ = input.handle_input(bounds, WidgetInput::TextEdit(TextEditCommand::SelectAll));
        assert_eq!(input.selected_text().as_deref(), Some("onetwo beta"));
        assert_eq!(
            input.handle_input(bounds, WidgetInput::TextEdit(TextEditCommand::CutSelection)),
            Some(TextInputMessage::Changed {
                value: String::new(),
            })
        );
    }
}
