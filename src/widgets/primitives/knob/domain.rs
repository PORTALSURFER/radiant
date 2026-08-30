//! Retained domain-space interaction for application-built knobs.

use std::rc::Rc;

use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::Rect,
    },
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        contract::{
            Widget, WidgetCapabilities, WidgetPointerMotion, WidgetPointerMotionRevision,
            WidgetSemantics,
        },
        interaction::{
            EditEvent, InteractionProvenance, KnobDomainCancellationReason, KnobDomainError,
            KnobDomainKeyboardGesture, KnobDomainMappingAttempt, KnobDomainMessage,
            KnobDomainWheelGesture, KnobKeyboardMetadata, KnobPointerMetadata, KnobWheelMetadata,
            NumericAdjustment, PointerButton, PointerModifiers, ValueFormat, WidgetInput,
            WidgetKey, WidgetOutput,
        },
    },
};

use super::{KnobWidget, WHEEL_FINE_STEP, WHEEL_STEP};
use crate::widgets::primitives::support::WidgetCommon;

/// Runtime-owned adapter for a Knob whose normalized interaction is projected
/// through an application-owned `f32` adjustment.
pub(crate) struct RetainedKnobDomainWidget<A> {
    pub(crate) knob: KnobWidget,
    active_edit: Option<EditEvent<f32>>,
    active_domain_start: Option<f32>,
    adjustment: Rc<A>,
    pub(crate) domain_value: f32,
    default_domain_value: f32,
    default_normalized_value: f32,
    value_format: Option<ValueFormat>,
}

impl<A> Clone for RetainedKnobDomainWidget<A> {
    fn clone(&self) -> Self {
        Self {
            knob: self.knob.clone(),
            active_edit: self.active_edit,
            active_domain_start: self.active_domain_start,
            adjustment: Rc::clone(&self.adjustment),
            domain_value: self.domain_value,
            default_domain_value: self.default_domain_value,
            default_normalized_value: self.default_normalized_value,
            value_format: self.value_format,
        }
    }
}

impl<A> RetainedKnobDomainWidget<A>
where
    A: crate::widgets::NumericAdjustment<f32>,
{
    pub(crate) fn new(
        knob: KnobWidget,
        adjustment: Rc<A>,
        domain_value: f32,
        default_domain_value: f32,
        default_normalized_value: f32,
    ) -> Self {
        Self {
            knob,
            active_edit: None,
            active_domain_start: None,
            adjustment,
            domain_value,
            default_domain_value,
            default_normalized_value,
            value_format: None,
        }
    }

    pub(crate) fn with_value_format(mut self, value_format: Option<ValueFormat>) -> Self {
        self.value_format = value_format;
        self
    }

    pub(super) fn handle_domain_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<KnobDomainMessage<A::Error>> {
        let previous = self.clone();
        let result = if self.active_edit.is_some()
            && !self.knob.is_editable()
            && !matches!(input, WidgetInput::FocusChanged(false))
        {
            Ok(self.cancel_active(
                KnobDomainCancellationReason::DisabledOrReadOnly,
                pointer_provenance_for_input(&input),
            ))
        } else {
            self.handle_domain_input_inner(bounds, input)
        };

        match result {
            Ok(message) => message,
            Err(failure) => {
                let retained_value = previous.domain_value;
                *self = previous;
                Some(KnobDomainMessage::MappingFailed {
                    attempt: failure.attempt,
                    normalized: failure.normalized,
                    retained_value,
                    provenance: failure.provenance,
                    error: failure.error,
                })
            }
        }
    }

    pub(super) fn handle_pointer_capture_cancelled(
        &mut self,
    ) -> Option<KnobDomainMessage<A::Error>> {
        self.knob.common.state.focused = false;
        self.cancel_active(
            KnobDomainCancellationReason::PointerCaptureLoss,
            pointer_provenance_empty(),
        )
    }

    fn handle_domain_input_inner(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Result<Option<KnobDomainMessage<A::Error>>, MappingFailure<A::Error>> {
        match input {
            WidgetInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
            } => {
                self.knob.common.state.hovered = bounds.contains(position);
                if !self.knob.is_editable()
                    || !self.knob.common.state.pressed
                    || self.active_edit.is_none()
                {
                    return Ok(None);
                }

                let origin = self.knob.state.gesture_origin.unwrap_or(position);
                self.knob.state.gesture_origin = Some(position);
                let sensitivity = if self.knob.state.fine_adjustment {
                    self.knob.props.sensitivity * 0.1
                } else {
                    self.knob.props.sensitivity
                };
                let Some(normalized) =
                    finite_clamped(self.knob.state.value + (origin.y - position.y) * sensitivity)
                else {
                    return Ok(None);
                };
                if !values_differ(self.knob.state.value, normalized) {
                    return Ok(None);
                }

                let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
                let value = self.map_candidate(
                    KnobDomainMappingAttempt::PointerUpdate,
                    normalized,
                    provenance,
                )?;
                let Some(previous) = self.active_edit else {
                    return Ok(None);
                };
                let Some(update) = previous.update(normalized, provenance) else {
                    return Ok(None);
                };
                self.knob.state.value = normalized;
                self.active_edit = Some(update);
                self.domain_value = value;
                Ok(Some(KnobDomainMessage::ValueChanged {
                    value,
                    metadata: pointer_metadata(provenance),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } if bounds.contains(position)
                && self.knob.is_editable()
                && !self.knob.common.state.pressed
                && self.active_edit.is_none() =>
            {
                self.knob.common.state.hovered = true;
                self.knob.common.state.pressed = true;
                self.knob.common.state.focused = true;
                self.knob.state.fine_adjustment = modifiers.shift;
                self.knob.state.gesture_origin = Some(position);
                let provenance = pointer_provenance(modifiers, timestamp, None);
                let begin = EditEvent::begin(self.knob.state.value, provenance);
                self.active_edit = Some(begin);
                self.active_domain_start = Some(self.domain_value);
                Ok(Some(KnobDomainMessage::GestureStarted {
                    value: self.domain_value,
                    metadata: pointer_metadata(provenance),
                }))
            }
            WidgetInput::PointerRelease {
                button: PointerButton::Primary,
                modifiers,
                timestamp,
                ..
            }
            | WidgetInput::PointerDrop {
                button: PointerButton::Primary,
                modifiers,
                timestamp,
                ..
            } => {
                let Some(previous) = self.active_edit else {
                    return Ok(None);
                };
                let provenance = pointer_provenance(modifiers, timestamp, None);
                if !self.knob.is_editable() {
                    return Ok(self.cancel_active(
                        KnobDomainCancellationReason::DisabledOrReadOnly,
                        provenance,
                    ));
                }
                if previous.commit(self.knob.state.value, provenance).is_none() {
                    return Ok(None);
                }
                let value = self.domain_value;
                self.active_edit = None;
                self.active_domain_start = None;
                clear_pointer_state(&mut self.knob);
                Ok(Some(KnobDomainMessage::GestureEnded {
                    value,
                    metadata: pointer_metadata(provenance),
                }))
            }
            WidgetInput::PointerModifiersChanged { modifiers, .. } => {
                if self.knob.common.state.pressed && self.active_edit.is_some() {
                    self.knob.state.fine_adjustment = modifiers.shift;
                }
                Ok(None)
            }
            WidgetInput::Wheel {
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            } if self.knob.is_editable()
                && self.active_edit.is_none()
                && !self.knob.common.state.pressed
                && self.knob.state.gesture_origin.is_none()
                && bounds.contains(position) =>
            {
                let direction = if delta.y > 0.0 {
                    1.0
                } else if delta.y < 0.0 {
                    -1.0
                } else {
                    return Ok(None);
                };
                let step = if modifiers.shift {
                    WHEEL_FINE_STEP
                } else {
                    WHEEL_STEP
                };
                let start_normalized = self.knob.state.value;
                let Some(normalized) = finite_clamped(start_normalized + direction * step) else {
                    return Ok(None);
                };
                if !values_differ(start_normalized, normalized) {
                    return Ok(None);
                }
                let provenance = pointer_provenance(modifiers, timestamp, sequence_range);
                let value = self.map_candidate(
                    KnobDomainMappingAttempt::WheelGesture,
                    normalized,
                    provenance,
                )?;
                let start_value = self.domain_value;
                self.knob.state.value = normalized;
                self.domain_value = value;
                Ok(Some(KnobDomainMessage::WheelGesture(
                    KnobDomainWheelGesture::new_with_metadata(
                        start_value,
                        value,
                        wheel_metadata(provenance),
                    ),
                )))
            }
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                modifiers,
                timestamp,
            } if self.knob.props.reset_on_double_click
                && self.knob.is_editable()
                && self.active_edit.is_none()
                && !self.knob.common.state.pressed
                && self.knob.state.gesture_origin.is_none()
                && bounds.contains(position) =>
            {
                let previous_value = self.domain_value;
                let provenance = pointer_provenance(modifiers, timestamp, None);
                let begin = EditEvent::begin(self.knob.state.value, provenance);
                let Some(update) = begin.update(self.default_normalized_value, provenance) else {
                    return Ok(None);
                };
                if update
                    .commit(self.default_normalized_value, provenance)
                    .is_none()
                {
                    return Ok(None);
                }
                self.knob.state.value = self.default_normalized_value;
                self.domain_value = self.default_domain_value;
                clear_pointer_state(&mut self.knob);
                Ok(Some(KnobDomainMessage::Reset {
                    previous_value,
                    value: self.default_domain_value,
                    metadata: pointer_metadata(provenance),
                }))
            }
            WidgetInput::FocusChanged(focused) => {
                self.knob.common.state.focused = focused;
                if focused {
                    Ok(None)
                } else {
                    Ok(self.cancel_active(
                        KnobDomainCancellationReason::FocusLoss,
                        pointer_provenance_empty(),
                    ))
                }
            }
            WidgetInput::KeyPress { key, timestamp, .. }
                if self.knob.common.state.focused
                    && self.knob.is_editable()
                    && self.active_edit.is_none()
                    && !self.knob.common.state.pressed
                    && self.knob.state.gesture_origin.is_none() =>
            {
                let Some(normalized) =
                    keyboard_candidate(self.knob.state.value, self.knob.props.sensitivity, key)
                else {
                    return Ok(None);
                };
                let start_normalized = self.knob.state.value;
                if !values_differ(start_normalized, normalized) {
                    return Ok(None);
                }
                let provenance = InteractionProvenance::Keyboard { timestamp };
                let value = self.map_candidate(
                    KnobDomainMappingAttempt::KeyboardGesture,
                    normalized,
                    provenance,
                )?;
                let start_value = self.domain_value;
                let begin = EditEvent::begin(start_normalized, provenance);
                let Some(update) = begin.update(normalized, provenance) else {
                    return Ok(None);
                };
                if update.commit(normalized, provenance).is_none() {
                    return Ok(None);
                }
                self.knob.state.value = normalized;
                self.domain_value = value;
                Ok(Some(KnobDomainMessage::KeyboardGesture(
                    KnobDomainKeyboardGesture::new_with_metadata(
                        start_value,
                        value,
                        KnobKeyboardMetadata { timestamp },
                    ),
                )))
            }
            _ => Ok(None),
        }
    }

    fn map_candidate(
        &self,
        attempt: KnobDomainMappingAttempt,
        normalized: f32,
        provenance: InteractionProvenance,
    ) -> Result<f32, MappingFailure<A::Error>> {
        validate_normalized(normalized).map_err(|error| MappingFailure {
            attempt,
            normalized,
            provenance,
            error,
        })?;
        let value = self
            .adjustment
            .normalized_to_value(normalized)
            .map_err(|error| MappingFailure {
                attempt,
                normalized,
                provenance,
                error: KnobDomainError::NormalizedToValue { error },
            })?;
        if !value.is_finite() {
            return Err(MappingFailure {
                attempt,
                normalized,
                provenance,
                error: KnobDomainError::NonFiniteValue { value },
            });
        }
        Ok(value)
    }

    fn cancel_active(
        &mut self,
        reason: KnobDomainCancellationReason,
        provenance: InteractionProvenance,
    ) -> Option<KnobDomainMessage<A::Error>> {
        let Some(previous) = self.active_edit.take() else {
            clear_pointer_state(&mut self.knob);
            self.active_domain_start = None;
            return None;
        };
        let start_value = self.active_domain_start.unwrap_or(self.domain_value);
        let previous_value = self.domain_value;
        let cancel = previous
            .cancel(provenance)
            .or_else(|| previous.cancel(pointer_provenance_empty()));
        self.knob.state.value = previous.start_value;
        self.domain_value = start_value;
        self.active_domain_start = None;
        clear_pointer_state(&mut self.knob);
        cancel.map(|_| KnobDomainMessage::GestureCancelled {
            start_value,
            previous_value,
            reason,
            metadata: pointer_metadata(provenance),
        })
    }
}

impl<A> WidgetSemantics for RetainedKnobDomainWidget<A>
where
    A: crate::widgets::NumericAdjustment<f32>,
{
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        let fallback = || format!("{:.3}", self.domain_value);
        let Some(value_format) = self.value_format else {
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

impl<A> WidgetPointerMotion for RetainedKnobDomainWidget<A>
where
    A: NumericAdjustment<f32>,
{
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(true)
    }
}

impl<A> Widget for RetainedKnobDomainWidget<A>
where
    A: crate::widgets::NumericAdjustment<f32> + 'static,
    A::Error: 'static,
{
    fn common(&self) -> &WidgetCommon {
        &self.knob.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.knob.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_domain_input(bounds, input)
            .map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        self.handle_pointer_capture_cancelled()
            .map(WidgetOutput::typed)
    }

    fn prepare_replacement(&mut self, successor: Option<&dyn Widget>) -> Option<WidgetOutput> {
        let successor =
            successor.and_then(|successor| successor.as_any().downcast_ref::<Self>())?;
        if !successor.knob.common.state.disabled && !successor.knob.common.state.read_only {
            return None;
        }
        self.active_edit.as_ref()?;
        self.cancel_active(
            KnobDomainCancellationReason::DisabledOrReadOnly,
            pointer_provenance_empty(),
        )
        .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };

        self.knob.common.state.hovered = previous.knob.common.state.hovered;
        self.knob.common.state.focused = previous.knob.common.state.focused;
        if self.knob.common.state.disabled || self.knob.common.state.read_only {
            clear_pointer_state(&mut self.knob);
            self.active_edit = None;
            self.active_domain_start = None;
        } else {
            self.knob.common.state.pressed = previous.knob.common.state.pressed;
            self.knob.state.fine_adjustment = previous.knob.state.fine_adjustment;
            self.knob.state.gesture_origin = previous.knob.state.gesture_origin;
            self.active_edit = previous.active_edit;
            self.active_domain_start = previous.active_domain_start;
        }
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new()
            .semantics(self)
            .pointer_motion(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.knob.append_paint(primitives, bounds, layout, theme);
    }
}

struct MappingFailure<E> {
    attempt: KnobDomainMappingAttempt,
    normalized: f32,
    provenance: InteractionProvenance,
    error: KnobDomainError<E>,
}

pub(crate) fn initial_normalized<A>(
    value: f32,
    adjustment: &A,
) -> Result<f32, KnobDomainError<A::Error>>
where
    A: crate::widgets::NumericAdjustment<f32>,
{
    if !value.is_finite() {
        return Err(KnobDomainError::NonFiniteValue { value });
    }
    let normalized = adjustment
        .value_to_normalized(&value)
        .map_err(|error| KnobDomainError::ValueToNormalized { error })?;
    validate_normalized(normalized)?;
    Ok(normalized)
}

fn validate_normalized<E>(normalized: f32) -> Result<(), KnobDomainError<E>> {
    if !normalized.is_finite() {
        return Err(KnobDomainError::NonFiniteNormalized { normalized });
    }
    if !(0.0..=1.0).contains(&normalized) {
        return Err(KnobDomainError::NormalizedOutOfRange { normalized });
    }
    Ok(())
}

fn keyboard_candidate(value: f32, sensitivity: f32, key: WidgetKey) -> Option<f32> {
    let candidate = match key {
        WidgetKey::ArrowLeft | WidgetKey::ArrowDown => value - sensitivity * 16.0,
        WidgetKey::ArrowRight | WidgetKey::ArrowUp => value + sensitivity * 16.0,
        WidgetKey::Home => 0.0,
        WidgetKey::End => 1.0,
        _ => return None,
    };
    finite_clamped(candidate)
}

fn finite_clamped(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.clamp(0.0, 1.0))
}

fn clear_pointer_state(knob: &mut KnobWidget) {
    knob.common.state.pressed = false;
    knob.state.fine_adjustment = false;
    knob.state.gesture_origin = None;
}

fn pointer_provenance(
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers,
        timestamp,
        sequence_range,
    }
}

fn pointer_provenance_empty() -> InteractionProvenance {
    pointer_provenance(PointerModifiers::default(), None, None)
}

fn pointer_provenance_for_input(input: &WidgetInput) -> InteractionProvenance {
    match input {
        WidgetInput::PointerMove {
            modifiers,
            timestamp,
            sequence_range,
            ..
        }
        | WidgetInput::Wheel {
            modifiers,
            timestamp,
            sequence_range,
            ..
        } => pointer_provenance(*modifiers, *timestamp, *sequence_range),
        WidgetInput::PointerPress {
            modifiers,
            timestamp,
            ..
        }
        | WidgetInput::PointerDoubleClick {
            modifiers,
            timestamp,
            ..
        }
        | WidgetInput::PointerRelease {
            modifiers,
            timestamp,
            ..
        }
        | WidgetInput::PointerDrop {
            modifiers,
            timestamp,
            ..
        } => pointer_provenance(*modifiers, *timestamp, None),
        WidgetInput::PointerModifiersChanged {
            modifiers,
            timestamp,
        } => pointer_provenance(*modifiers, *timestamp, None),
        WidgetInput::KeyPress { timestamp, .. } => InteractionProvenance::Keyboard {
            timestamp: *timestamp,
        },
        _ => pointer_provenance_empty(),
    }
}

fn pointer_metadata(provenance: InteractionProvenance) -> KnobPointerMetadata {
    match provenance {
        InteractionProvenance::Pointer {
            modifiers,
            timestamp,
            sequence_range,
        } => KnobPointerMetadata {
            modifiers,
            timestamp,
            sequence_range,
        },
        _ => KnobPointerMetadata::empty(),
    }
}

fn wheel_metadata(provenance: InteractionProvenance) -> KnobWheelMetadata {
    match provenance {
        InteractionProvenance::Pointer {
            modifiers,
            timestamp,
            sequence_range,
        } => KnobWheelMetadata {
            modifiers,
            timestamp,
            sequence_range,
        },
        _ => KnobWheelMetadata::empty(),
    }
}

fn values_differ(left: f32, right: f32) -> bool {
    if left.is_finite() && right.is_finite() {
        (left - right).abs() > f32::EPSILON
    } else {
        left != right
    }
}

#[cfg(test)]
#[path = "domain_tests.rs"]
mod tests;
