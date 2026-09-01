//! Crate-private retained adapter for Slider's typed edit lifecycle.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{
    Widget, WidgetCapabilities, WidgetPointerMotion, WidgetPointerMotionRevision, WidgetSemantics,
};
use crate::widgets::interaction::{
    EditEvent, NumericAdjustment, PointerButton, SliderDomainError, SliderDomainMessage,
    SliderEditBatch, ValueFormat, WidgetInput, WidgetOutput,
};
use std::rc::Rc;

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

/// Runtime-owned adapter for a Slider whose normalized value is projected
/// through an application-owned `f32` adjustment.
pub(crate) struct RetainedSliderDomainWidget<A> {
    pub(crate) slider: RetainedSliderWidget,
    adjustment: Rc<A>,
    domain_value: f32,
}

impl<A> Clone for RetainedSliderDomainWidget<A> {
    fn clone(&self) -> Self {
        Self {
            slider: self.slider.clone(),
            adjustment: Rc::clone(&self.adjustment),
            domain_value: self.domain_value,
        }
    }
}

impl<A> RetainedSliderDomainWidget<A>
where
    A: NumericAdjustment<f32>,
{
    pub(crate) fn new(slider: SliderWidget, adjustment: Rc<A>, domain_value: f32) -> Self {
        Self {
            slider: RetainedSliderWidget::new(slider),
            adjustment,
            domain_value,
        }
    }

    pub(crate) fn with_value_format(mut self, value_format: Option<ValueFormat>) -> Self {
        self.slider = self.slider.with_value_format(value_format);
        self
    }

    pub(super) fn handle_domain_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<SliderDomainMessage<A::Error>> {
        let terminal_input = matches!(
            &input,
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                ..
            } | WidgetInput::FocusChanged(false)
        );
        let previous_value = self.slider.slider.state.value;
        let previous_state = self.slider.slider.common.state;
        let previous_active_edit = self.slider.active_edit;
        let batch = self.slider.handle_edit_input(bounds, input)?;
        let normalized = batch.value_change()?;

        match self.map_normalized(normalized) {
            Ok(value) => {
                self.domain_value = value;
                Some(SliderDomainMessage::ValueChanged { value })
            }
            Err(error) => {
                self.slider.slider.state.value = previous_value;
                if !terminal_input {
                    self.slider.slider.common.state = previous_state;
                    self.slider.active_edit = previous_active_edit;
                }
                Some(SliderDomainMessage::MappingFailed { normalized, error })
            }
        }
    }

    pub(super) fn handle_pointer_capture_cancelled(
        &mut self,
    ) -> Option<SliderDomainMessage<A::Error>> {
        let previous_value = self.slider.slider.state.value;
        let batch = input::handle_pointer_capture_cancelled(
            &mut self.slider.slider,
            &mut self.slider.active_edit,
        )?;
        let normalized = batch.value_change()?;
        match self.map_normalized(normalized) {
            Ok(value) => {
                self.domain_value = value;
                Some(SliderDomainMessage::ValueChanged { value })
            }
            Err(error) => {
                self.slider.slider.state.value = previous_value;
                Some(SliderDomainMessage::MappingFailed { normalized, error })
            }
        }
    }

    fn map_normalized(&self, normalized: f32) -> Result<f32, SliderDomainError<A::Error>> {
        validate_normalized(normalized)?;
        let value = self
            .adjustment
            .normalized_to_value(normalized)
            .map_err(|error| SliderDomainError::NormalizedToValue { error })?;
        value
            .is_finite()
            .then_some(value)
            .ok_or(SliderDomainError::NonFiniteValue { value })
    }
}

pub(crate) fn initial_normalized<A>(
    value: f32,
    adjustment: &A,
) -> Result<f32, SliderDomainError<A::Error>>
where
    A: NumericAdjustment<f32>,
{
    if !value.is_finite() {
        return Err(SliderDomainError::NonFiniteValue { value });
    }
    let normalized = adjustment
        .value_to_normalized(&value)
        .map_err(|error| SliderDomainError::ValueToNormalized { error })?;
    validate_normalized(normalized)?;
    Ok(normalized)
}

fn validate_normalized<E>(normalized: f32) -> Result<(), SliderDomainError<E>> {
    if !normalized.is_finite() {
        return Err(SliderDomainError::NonFiniteNormalized { normalized });
    }
    if !(0.0..=1.0).contains(&normalized) {
        return Err(SliderDomainError::NormalizedOutOfRange { normalized });
    }
    Ok(())
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

impl WidgetPointerMotion for RetainedSliderWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(true)
    }
}

impl<A> WidgetSemantics for RetainedSliderDomainWidget<A>
where
    A: NumericAdjustment<f32>,
{
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        let fallback = || format!("{:.3}", self.domain_value);
        let Some(value_format) = self.slider.value_format else {
            return Some(fallback());
        };

        let mut output = String::new();
        if value_format
            .write_into(self.domain_value, &mut output)
            .is_ok()
        {
            Some(output)
        } else {
            Some(fallback())
        }
    }
}

impl<A> WidgetPointerMotion for RetainedSliderDomainWidget<A>
where
    A: NumericAdjustment<f32>,
{
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(true)
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

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        input::handle_pointer_capture_cancelled(&mut self.slider, &mut self.active_edit)
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

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
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

impl<A> Widget for RetainedSliderDomainWidget<A>
where
    A: NumericAdjustment<f32> + 'static,
    A::Error: 'static,
{
    fn common(&self) -> &WidgetCommon {
        self.slider.common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.slider.common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_domain_input(bounds, input)
            .map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        self.handle_pointer_capture_cancelled()
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.slider.synchronize_from_previous(&previous.slider);
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.slider.append_paint(primitives, bounds, layout, theme);
    }
}
