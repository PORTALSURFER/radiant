//! Crate-private retained adapter for Slider's typed edit lifecycle.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{Widget, WidgetCapabilities, WidgetSemantics};
use crate::widgets::interaction::{
    EditEvent, SliderEditBatch, ValueFormat, WidgetInput, WidgetOutput,
};

use super::{SliderWidget, input, paint};
use crate::widgets::primitives::support::WidgetCommon;

/// Runtime-owned adapter for official Slider projections.
///
/// The public [`SliderWidget`] remains a three-field source-compatible widget.
/// This adapter is the only owner of the active typed edit transaction used by
/// the runtime/application Slider constructors.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RetainedSliderWidget {
    pub(crate) slider: SliderWidget,
    active_edit: Option<EditEvent<f32>>,
    value_format: Option<ValueFormat>,
}

impl RetainedSliderWidget {
    pub(crate) fn new(slider: SliderWidget) -> Self {
        Self {
            slider,
            active_edit: None,
            value_format: None,
        }
    }

    pub(crate) fn with_value_format(mut self, value_format: Option<ValueFormat>) -> Self {
        self.value_format = value_format;
        self
    }

    pub(super) fn handle_edit_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<SliderEditBatch> {
        input::handle_slider_edit_input(&mut self.slider, &mut self.active_edit, bounds, input)
    }
}

impl WidgetSemantics for RetainedSliderWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        let fallback = || format!("{:.3}", self.slider.state.value);
        let Some(value_format) = self.value_format else {
            return Some(fallback());
        };

        let mut output = String::new();
        if value_format
            .write_into(self.slider.state.value, &mut output)
            .is_ok()
        {
            Some(output)
        } else {
            Some(fallback())
        }
    }
}

impl Widget for RetainedSliderWidget {
    fn common(&self) -> &WidgetCommon {
        &self.slider.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.slider.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_edit_input(bounds, input)
            .map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        self.handle_edit_input(bounds, WidgetInput::FocusChanged(false))
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };

        // The fresh projection remains authoritative for value, props, sizing,
        // style, and semantics. Only runtime-owned interaction state crosses a
        // compatible same-wrapper Slider refresh.
        self.slider.common.state.hovered = previous.slider.common.state.hovered;
        self.slider.common.state.focused = previous.slider.common.state.focused;
        if self.slider.common.state.disabled || self.slider.common.state.read_only {
            self.slider.common.state.pressed = false;
            self.active_edit = None;
        } else {
            self.slider.common.state.pressed = previous.slider.common.state.pressed;
            self.active_edit = previous.active_edit;
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
        paint::push_slider_widget_paint(primitives, &self.slider, bounds, theme);
    }
}
