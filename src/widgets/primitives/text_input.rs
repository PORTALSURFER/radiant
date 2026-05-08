//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::WidgetCommon;
use super::support::push_text_input_widget_paint;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{PointerButton, TextInputMessage, WidgetInput, WidgetOutput};

mod editing;
use editing::caret_for_pointer_x;

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

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};
    use crate::widgets::interaction::{TextEditCommand, WidgetKey};

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
