//! Reusable toggle primitive.

mod builders;
mod input;
mod model;
mod paint;

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::{
    WidgetCommon,
    revision::{
        CommonInteractionRevision, CommonPaintRevision, common_geometry, common_interaction,
        common_paint, exact_revision,
    },
};
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetCapabilities, WidgetId, WidgetPointerMotion,
    WidgetPointerMotionRevision, WidgetRevision, WidgetSemantics, WidgetSemanticsRevision,
    WidgetSizing,
};
use crate::widgets::interaction::{
    InteractionProvenance, ToggleMessage, WidgetInput, WidgetOutput,
};

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
    pub(super) fn toggle(&mut self, provenance: InteractionProvenance) -> ToggleMessage {
        self.state.checked = !self.state.checked;
        self.common.state.active = self.state.checked;
        ToggleMessage::ValueChanged {
            checked: self.state.checked,
            provenance,
        }
    }
}

impl WidgetSemantics for ToggleWidget {
    fn revision(&self) -> WidgetSemanticsRevision {
        WidgetSemanticsRevision::exact((
            self.props.label.clone(),
            self.state.checked,
            self.common.state.selected,
            self.common.state.disabled,
            self.common.state.read_only,
        ))
    }

    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Toggle
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.props.label.as_str().to_owned())
    }

    fn automation_checked(&self) -> Option<bool> {
        Some(self.state.checked)
    }
}

impl WidgetPointerMotion for ToggleWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TogglePaintRevision {
    common: CommonPaintRevision,
    label: PaintText,
    checked: bool,
    active: bool,
    selected: bool,
    disabled: bool,
    automation_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToggleInteractionRevision {
    common: CommonInteractionRevision,
    checked: bool,
    paints_state_layers: bool,
    suppresses_container_hover: bool,
    disabled: bool,
    read_only: bool,
}

impl ToggleWidget {
    fn exact_revision(&self) -> Option<WidgetRevision> {
        exact_revision(
            common_geometry(&self.common),
            TogglePaintRevision {
                common: common_paint(&self.common),
                label: self.props.label.clone(),
                checked: self.state.checked,
                active: self.common.state.active,
                selected: self.common.state.selected,
                disabled: self.common.state.disabled,
                automation_active: self.common.state.automation_active,
            },
            ToggleInteractionRevision {
                common: common_interaction(&self.common),
                checked: self.state.checked,
                paints_state_layers: self.common.paint.paints_state_layers,
                suppresses_container_hover: self.common.paint.suppresses_container_hover,
                disabled: self.common.state.disabled,
                read_only: self.common.state.read_only,
            },
        )
    }
}

impl Widget for ToggleWidget {
    fn revision(&self) -> WidgetRevision {
        self.exact_revision()
            .unwrap_or_else(WidgetRevision::conservative)
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ToggleWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new()
            .semantics(self)
            .pointer_motion(self)
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
