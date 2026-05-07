//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::WidgetCommon;
use super::support::push_text_input_widget_paint;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    PointerButton, TextEditCommand, TextInputMessage, WidgetInput, WidgetKey, WidgetOutput,
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
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
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
                if self.common.state.pressed {
                    self.set_caret(caret_for_pointer_x(bounds, position.x), true);
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.focused = true;
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.set_caret(caret_for_pointer_x(bounds, position.x), false);
                None
            }
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } => {
                self.common.state.pressed = false;
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
        if !self.has_selection() {
            return (self.state.caret, self.state.caret);
        }
        let start = self.state.selection_anchor.min(self.state.caret);
        let end = self
            .state
            .selection_anchor
            .max(self.state.caret)
            .saturating_add(1)
            .min(self.char_len());
        (start, end)
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

impl Widget for TextInputWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        TextInputWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_text_input_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a text-input-message mapper.
    pub fn text_input(map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a single-line text input that maps edits and submissions by value.
    pub fn text_input(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::text_input_mapped(id, value, sizing, move |message| match message {
            TextInputMessage::Changed { value } | TextInputMessage::Submitted { value } => {
                map(value)
            }
        })
    }

    /// Build a single-line text input with a custom widget-to-host message mapper.
    pub fn text_input_mapped(
        id: WidgetId,
        value: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(TextInputMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            TextInputWidget::new(id, value, sizing),
            WidgetMessageMapper::text_input(map),
        )
    }
}

fn byte_index_for_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

fn caret_for_pointer_x(bounds: Rect, x: f32) -> usize {
    let text_x = (x - bounds.min.x - 16.0).max(0.0);
    let font_size: f32 = if bounds.height() >= 42.0 { 15.0 } else { 13.0 };
    let char_width = (font_size * 0.58_f32).max(1.0_f32);
    (text_x / char_width).floor().max(0.0) as usize
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
        for _ in 0..4 {
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

    #[test]
    fn text_input_pointer_drag_extends_selection_including_caret_character() {
        let mut input = TextInputWidget::new(
            7,
            "abcdef",
            WidgetSizing::new(Vector2::new(100.0, 42.0), Vector2::new(180.0, 42.0)),
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 42.0));

        assert_eq!(
            input.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(26.0, 20.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert_eq!(input.state.caret, 1);
        assert_eq!(
            input.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(43.0, 20.0),
                },
            ),
            None
        );
        assert_eq!(input.state.caret, 3);
        assert_eq!(input.selected_text().as_deref(), Some("bcd"));
        assert_eq!(
            input.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(43.0, 20.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert!(!input.common.state.pressed);
    }
}
