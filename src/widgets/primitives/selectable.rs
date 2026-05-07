//! Reusable selectable surface primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, activate_on_keyboard, push_selectable_widget_paint};
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{PointerButton, SelectableMessage, WidgetInput, WidgetOutput};

/// Immutable public properties for a reusable selectable surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableProps {
    /// User-visible selectable label.
    pub label: String,
}

/// Public selectable primitive for cards, rows, tiles, and options.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing selectable configuration.
    pub props: SelectableProps,
}

impl SelectableWidget {
    /// Build a selectable descriptor with the provided selected state.
    pub fn new(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
    ) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = selected;
        Self {
            common,
            props: SelectableProps {
                label: label.into(),
            },
        }
    }

    /// Route one backend-neutral interaction into the selectable.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SelectableMessage> {
        if self.common.state.disabled {
            return None;
        }

        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.pressed = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let was_pressed = self.common.state.pressed;
                self.common.state.pressed = false;
                (was_pressed && bounds.contains(position)).then(|| self.toggle_selected())
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused && activate_on_keyboard(key) =>
            {
                Some(self.toggle_selected())
            }
            _ => None,
        }
    }

    fn toggle_selected(&mut self) -> SelectableMessage {
        self.common.state.selected = !self.common.state.selected;
        SelectableMessage::SelectionChanged {
            selected: self.common.state.selected,
        }
    }
}

impl Widget for SelectableWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SelectableWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_selectable_widget_paint(primitives, self, bounds, theme);
    }
}
