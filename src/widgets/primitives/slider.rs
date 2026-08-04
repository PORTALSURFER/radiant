//! Reusable horizontal slider primitive.

mod builders;
mod geometry;
mod input;
mod model;
mod paint;

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, clamp_fraction};
use crate::widgets::contract::{
    FocusBehavior, PaintBounds, Widget, WidgetCapabilities, WidgetId, WidgetSemantics, WidgetSizing,
};
use crate::widgets::interaction::{SliderEditBatch, SliderMessage, WidgetInput, WidgetOutput};

pub use model::{SliderProps, SliderState};

const DEFAULT_KEYBOARD_STEP: f32 = 0.05;
const DEFAULT_TRACK_HEIGHT: f32 = 6.0;

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
                track_height: DEFAULT_TRACK_HEIGHT,
                paints_track_border: false,
            },
            state: SliderState {
                value: clamp_fraction(parts.value),
                active_edit: None,
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

    /// Return this slider with an explicit centered track height.
    pub fn with_track_height(mut self, height: f32) -> Self {
        self.props.track_height = height;
        self
    }

    /// Return this slider with a passive one-pixel track outline.
    pub fn with_track_border(mut self, paints_border: bool) -> Self {
        self.props.paints_track_border = paints_border;
        self
    }

    /// Return the current thumb rectangle inside the provided bounds.
    pub fn thumb_rect(&self, bounds: Rect) -> Rect {
        geometry::thumb_rect(bounds, self.state.value, self.props.track_height)
    }

    /// Route one backend-neutral interaction into the slider.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SliderMessage> {
        self.handle_edit_input(bounds, input)
            .and_then(|batch| batch.value_change())
            .map(|value| SliderMessage::ValueChanged { value })
    }

    /// Route one backend-neutral interaction into the complete typed edit
    /// lifecycle emitted by the slider.
    pub fn handle_edit_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<SliderEditBatch> {
        input::handle_slider_edit_input(self, bounds, input)
    }

    pub(super) fn set_value_if_changed(&mut self, value: f32) -> bool {
        if (self.state.value - value).abs() <= f32::EPSILON {
            return false;
        }
        self.state.value = value;
        true
    }

    pub(super) fn is_editable(&self) -> bool {
        !self.common.state.disabled && !self.common.state.read_only
    }
}

impl WidgetSemantics for SliderWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(format!("{:.3}", self.state.value))
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
        SliderWidget::handle_edit_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        SliderWidget::handle_edit_input(self, bounds, WidgetInput::FocusChanged(false))
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        // The fresh projection remains authoritative for value, props, sizing,
        // style, and semantics. Only runtime-owned interaction state crosses a
        // compatible same-ID Slider refresh.
        self.common.state.hovered = previous.common.state.hovered;
        self.common.state.focused = previous.common.state.focused;
        if self.common.state.disabled || self.common.state.read_only {
            self.common.state.pressed = false;
            self.state.active_edit = None;
        } else {
            self.common.state.pressed = previous.common.state.pressed;
            self.state.active_edit = previous.state.active_edit;
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
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
