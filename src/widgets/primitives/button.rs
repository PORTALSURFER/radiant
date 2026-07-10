//! Reusable button primitive.

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
use crate::widgets::TextAlign;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ButtonMessage, WidgetInput, WidgetOutput};

pub use model::{ButtonProps, ButtonState};

/// Public button primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing button configuration.
    pub props: ButtonProps,
    /// Mutable interaction state owned by the button.
    pub state: ButtonState,
}

/// Named construction fields for a [`ButtonWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidgetParts {
    /// Stable widget id used by layout, paint, and input routing.
    pub id: WidgetId,
    /// User-facing button label.
    pub label: PaintText,
    /// Intrinsic sizing contract for the button.
    pub sizing: WidgetSizing,
}

impl ButtonWidget {
    /// Build a button descriptor from named parts.
    pub fn from_parts(parts: ButtonWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ButtonProps {
                label: parts.label,
                trailing_label: None,
                text_align: TextAlign::Center,
                secondary_click: false,
                drag: false,
                hover_chrome_only: false,
            },
            state: ButtonState::default(),
        }
    }

    /// Build a button descriptor with keyboard focus and activation semantics.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ButtonWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
    }

    /// Enable secondary/right-click activation messages for this button.
    pub fn with_secondary_click(mut self) -> Self {
        self.props.secondary_click = true;
        self
    }

    /// Enable primary-pointer drag lifecycle messages from the button surface.
    pub fn with_drag(mut self) -> Self {
        self.props.drag = true;
        self
    }

    /// Paint button chrome only while hovered, pressed, or focused.
    pub fn with_hover_chrome_only(mut self) -> Self {
        self.props.hover_chrome_only = true;
        self
    }

    /// Add passive trailing text while preserving the main label storage.
    pub fn with_trailing_label(mut self, label: impl Into<PaintText>) -> Self {
        self.props.trailing_label = Some(label.into());
        self
    }

    /// Route one backend-neutral interaction into the button.
    ///
    /// The button emits [`ButtonMessage::Activate`] when a primary press is
    /// released inside bounds or when the focused widget receives Enter/Space.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ButtonMessage> {
        input::handle_button_input(self, bounds, input)
    }
}

impl Widget for ButtonWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ButtonWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.state = previous.state;
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Button
    }

    fn automation_label(&self) -> Option<String> {
        Some(match self.props.trailing_label.as_ref() {
            Some(trailing) => format!("{} {}", self.props.label, trailing),
            None => self.props.label.as_str().to_owned(),
        })
    }

    fn needs_state_synchronization(&self) -> bool {
        true
    }

    fn set_text_align(&mut self, align: TextAlign) -> bool {
        self.props.text_align = align;
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_button_widget_paint(primitives, self, bounds, theme);
    }
}

#[cfg(test)]
mod tests;
