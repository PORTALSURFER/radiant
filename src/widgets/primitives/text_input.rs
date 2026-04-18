//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;

use super::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, WidgetId, WidgetKind, WidgetMessageKind, WidgetSizing,
};
use crate::widgets::interaction::{PointerButton, TextInputMessage, WidgetInput, WidgetKey};

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
}

impl TextInputState {
    fn new(value: String) -> Self {
        let caret = value.chars().count();
        Self { value, caret }
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
                self.state.caret = self.state.value.chars().count();
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
                self.insert_character(ch)
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused
                    && !self.common.state.disabled
                    && !self.common.state.read_only =>
            {
                self.handle_key_input(key)
            }
            _ => None,
        }
    }

    fn handle_key_input(&mut self, key: WidgetKey) -> Option<TextInputMessage> {
        match key {
            WidgetKey::ArrowLeft => {
                self.state.caret = self.state.caret.saturating_sub(1);
                None
            }
            WidgetKey::ArrowRight => {
                self.state.caret = (self.state.caret + 1).min(self.state.value.chars().count());
                None
            }
            WidgetKey::Home => {
                self.state.caret = 0;
                None
            }
            WidgetKey::End => {
                self.state.caret = self.state.value.chars().count();
                None
            }
            WidgetKey::Backspace => self.delete_before_caret(),
            WidgetKey::Delete => self.delete_after_caret(),
            WidgetKey::Enter if self.props.submit_on_enter => Some(TextInputMessage::Submitted {
                value: self.state.value.clone(),
            }),
            _ => None,
        }
    }

    fn insert_character(&mut self, ch: char) -> Option<TextInputMessage> {
        if self
            .props
            .character_limit
            .is_some_and(|limit| self.state.value.chars().count() >= limit)
        {
            return None;
        }
        let byte_index = byte_index_for_char(&self.state.value, self.state.caret);
        self.state.value.insert(byte_index, ch);
        self.state.caret += 1;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn delete_before_caret(&mut self) -> Option<TextInputMessage> {
        if self.state.caret == 0 {
            return None;
        }
        let end = byte_index_for_char(&self.state.value, self.state.caret);
        let start = byte_index_for_char(&self.state.value, self.state.caret - 1);
        self.state.value.replace_range(start..end, "");
        self.state.caret -= 1;
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }

    fn delete_after_caret(&mut self) -> Option<TextInputMessage> {
        if self.state.caret >= self.state.value.chars().count() {
            return None;
        }
        let start = byte_index_for_char(&self.state.value, self.state.caret);
        let end = byte_index_for_char(&self.state.value, self.state.caret + 1);
        self.state.value.replace_range(start..end, "");
        Some(TextInputMessage::Changed {
            value: self.state.value.clone(),
        })
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
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
}
