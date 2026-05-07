//! Reusable list-row and list-item primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, activate_on_keyboard, push_list_item_widget_paint};
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ListItemMessage, PointerButton, WidgetInput, WidgetOutput};

/// Public list-row or list-item primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Primary row label.
    pub label: String,
    /// Optional secondary text.
    pub detail: Option<String>,
}

impl ListItemWidget {
    /// Build a list-item descriptor that can be focused, selected, and invoked.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            label: label.into(),
            detail: None,
        }
    }

    /// Route one backend-neutral interaction into the list item.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ListItemMessage> {
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
                (was_pressed && bounds.contains(position)).then_some(ListItemMessage::Invoked)
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key)
                if self.common.state.focused && activate_on_keyboard(key) =>
            {
                Some(ListItemMessage::Invoked)
            }
            _ => None,
        }
    }
}

impl Widget for ListItemWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ListItemWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_list_item_widget_paint(primitives, self, bounds, theme);
    }
}
