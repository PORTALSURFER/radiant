//! Reusable toggle primitive.

mod builders;
mod input;
mod model;
mod paint;

use crate::gui::automation::AutomationRole;
use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ToggleMessage, WidgetInput, WidgetOutput};

pub use model::{ToggleProps, ToggleState};

/// Public toggle primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing toggle configuration.
    pub props: ToggleProps,
    /// Mutable interaction state owned by the toggle.
    pub state: ToggleState,
}

/// Named construction fields for a [`ToggleWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleWidgetParts {
    /// Stable widget id used by layout, paint, and input routing.
    pub id: WidgetId,
    /// User-facing toggle label.
    pub label: PaintText,
    /// Intrinsic sizing contract for the toggle.
    pub sizing: WidgetSizing,
}

impl ToggleWidget {
    /// Build a toggle descriptor from named parts.
    pub fn from_parts(parts: ToggleWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ToggleProps { label: parts.label },
            state: ToggleState::default(),
        }
    }

    /// Build a toggle descriptor with value-change semantics.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ToggleWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
    }

    /// Return this toggle with an explicit checked value.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self.common.state.active = checked;
        self
    }

    /// Route one backend-neutral interaction into the toggle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ToggleMessage> {
        input::handle_toggle_input(self, bounds, input)
    }
}

impl ToggleWidget {
    pub(super) fn toggle(&mut self) -> ToggleMessage {
        self.state.checked = !self.state.checked;
        self.common.state.active = self.state.checked;
        ToggleMessage::ValueChanged {
            checked: self.state.checked,
        }
    }
}

impl Widget for ToggleWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ToggleWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Toggle
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.props.label.as_str().to_owned())
    }

    fn automation_checked(&self) -> Option<bool> {
        Some(self.state.checked)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state.hovered = previous.common.state.hovered;
        self.common.state.pressed = previous.common.state.pressed;
        self.common.state.focused = previous.common.state.focused;
        self.state.armed = previous.state.armed;
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_toggle_widget_paint(primitives, self, bounds, theme);
    }
}

#[cfg(test)]
#[path = "toggle/tests.rs"]
mod tests;
