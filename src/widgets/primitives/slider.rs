//! Reusable horizontal slider primitive.

mod builders;
mod geometry;
mod input;
mod model;
mod paint;

use crate::gui::automation::AutomationRole;
use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, clamp_fraction};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{SliderMessage, WidgetInput, WidgetOutput};

pub use model::{SliderProps, SliderState};

const DEFAULT_KEYBOARD_STEP: f32 = 0.05;

/// Public horizontal slider primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable slider configuration.
    pub props: SliderProps,
    /// Mutable slider state owned by the widget.
    pub state: SliderState,
}

/// Named construction fields for [`SliderWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct SliderWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Initial normalized slider value.
    pub value: f32,
    /// Intrinsic slider sizing contract.
    pub sizing: WidgetSizing,
}

impl SliderWidget {
    /// Build a horizontal slider from named identity, value, and sizing fields.
    pub fn from_parts(parts: SliderWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            props: SliderProps {
                keyboard_step: DEFAULT_KEYBOARD_STEP,
            },
            state: SliderState {
                value: clamp_fraction(parts.value),
            },
        }
    }

    /// Build a horizontal slider with normalized value-change semantics.
    pub fn new(id: WidgetId, value: f32, sizing: WidgetSizing) -> Self {
        Self::from_parts(SliderWidgetParts { id, value, sizing })
    }

    /// Return this slider with an explicit normalized value.
    pub fn with_value(mut self, value: f32) -> Self {
        self.state.value = clamp_fraction(value);
        self
    }

    /// Return the current thumb rectangle inside the provided bounds.
    pub fn thumb_rect(&self, bounds: Rect) -> Rect {
        geometry::thumb_rect(bounds, self.state.value)
    }

    /// Route one backend-neutral interaction into the slider.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SliderMessage> {
        input::handle_slider_input(self, bounds, input)
    }

    pub(super) fn set_value(&mut self, value: f32) -> Option<SliderMessage> {
        let value = clamp_fraction(value);
        if (self.state.value - value).abs() <= f32::EPSILON {
            return None;
        }
        self.state.value = value;
        Some(SliderMessage::ValueChanged { value })
    }
}

impl Widget for SliderWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SliderWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(format!("{:.3}", self.state.value))
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_slider_widget_paint(primitives, self, bounds, theme);
    }
}

#[cfg(test)]
#[path = "slider/tests.rs"]
mod tests;
