//! Reusable single-line text-input primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{TextInputMessage, WidgetInput, WidgetOutput};

mod builders;
mod editing;
mod editing_ops;
mod input;
mod model;
mod paint;

#[cfg(test)]
mod tests;

pub use model::{TextInputEditResult, TextInputProps, TextInputState};

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

/// Named construction fields for [`TextInputWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextInputWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Initial text value.
    pub value: String,
    /// Intrinsic text-input sizing contract.
    pub sizing: WidgetSizing,
}

impl TextInputWidget {
    /// Build a single-line text-input descriptor from named identity, value, and sizing fields.
    pub fn from_parts(parts: TextInputWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: TextInputProps {
                placeholder: None,
                submit_on_enter: true,
                character_limit: None,
            },
            state: TextInputState::from_value(parts.value),
        }
    }

    /// Build a single-line text-input descriptor with edit semantics.
    pub fn new(id: WidgetId, value: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::from_parts(TextInputWidgetParts {
            id,
            value: value.into(),
            sizing,
        })
    }

    /// Route one backend-neutral interaction into the single-line text input.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<TextInputMessage> {
        input::handle_text_input(self, bounds, input)
    }

    pub(super) fn accepts_editing_input(&self) -> bool {
        self.common.state.focused && !self.common.state.disabled && !self.common.state.read_only
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

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<TextInputWidget>()
            && self.state.value == previous.state.value
        {
            self.state = previous.state.clone();
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.accepts_editing_input()
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn selected_text_slice(&self) -> Option<&str> {
        self.selected_text_slice()
    }

    fn selected_text(&self) -> Option<String> {
        self.selected_text()
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_text_input_widget_paint(primitives, self, bounds, theme);
    }
}
