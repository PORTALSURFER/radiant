//! Complete-mode numeric wheel state and typed output policy.

use std::{marker::PhantomData, rc::Rc};

use crate::{
    gui::types::{Point, Rect},
    widgets::{
        InteractionProvenance, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputEditBatch, NumericInputInteraction, NumericInputInteractionBatch, NumericStep,
        NumericWheelAttempt, NumericWheelPolicy, PointerModifiers, WHEEL_LINE_EQUIVALENCE_PIXELS,
        WheelDelta, WheelSample, WidgetOutput,
    },
};

use super::pointer::{select_step, valid_geometry};

/// Retained state for one admitted exact wheel sequence.
#[derive(Clone)]
pub(super) struct WheelSequenceState<T> {
    pub(super) start_value: T,
    pub(super) start_text: String,
    pub(super) start_caret: usize,
    pub(super) start_selection_anchor: usize,
    pub(super) start_provenance: InteractionProvenance,
    pub(super) session: Option<NumericEditSession<T>>,
}

impl<T: Clone> WheelSequenceState<T> {
    pub(super) fn new(
        value: T,
        text: String,
        caret: usize,
        selection_anchor: usize,
        provenance: InteractionProvenance,
    ) -> Self {
        Self {
            start_value: value,
            start_text: text,
            start_caret: caret,
            start_selection_anchor: selection_anchor,
            start_provenance: provenance,
            session: None,
        }
    }

    pub(super) const fn is_active(&self) -> bool {
        self.session.is_some()
    }

    pub(super) fn begin_update(
        &mut self,
        value: T,
        provenance: InteractionProvenance,
    ) -> Option<NumericInputEditBatch<T>> {
        if self.session.is_none() {
            let session = NumericEditSession::begin(
                self.start_value.clone(),
                self.start_text.clone(),
                self.start_provenance,
            );
            let begin = session.begin_event().clone();
            let update = begin.clone().update(value, provenance)?;
            let edit = NumericInputEditBatch::from_events(&[begin, update])?;
            self.session = Some(session);
            Some(edit)
        } else {
            let begin = self.session.as_ref()?.begin_event().clone();
            let update = begin.update(value, provenance)?;
            NumericInputEditBatch::from_events(&[update])
        }
    }
}

/// A failure which delays typed output construction until the handler knows
/// whether an active transaction needs a rollback part.
pub(super) struct WheelFailure<T> {
    emit: Box<dyn FnOnce(Option<NumericInputEditBatch<T>>) -> Option<WidgetOutput>>,
}

impl<T> WheelFailure<T> {
    pub(super) fn new(
        emit: impl FnOnce(Option<NumericInputEditBatch<T>>) -> Option<WidgetOutput> + 'static,
    ) -> Self {
        Self {
            emit: Box::new(emit),
        }
    }

    pub(super) fn into_output(
        self,
        rollback: Option<NumericInputEditBatch<T>>,
    ) -> Option<WidgetOutput> {
        (self.emit)(rollback)
    }
}

/// One policy call made by an effective exact wheel sample.
pub(super) struct WheelRequest<'a, T> {
    pub(super) value: &'a T,
    pub(super) delta: f32,
    pub(super) step: NumericStep,
    pub(super) attempt: NumericWheelAttempt,
    pub(super) provenance: InteractionProvenance,
}

/// Complete-mode typed wheel policy boundary.
pub(super) trait WheelOutputPolicy<T> {
    fn wheel(&self, request: WheelRequest<'_, T>) -> Result<T, WheelFailure<T>>;

    fn format(
        &self,
        request: WheelRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), WheelFailure<T>>;
}

struct CompleteWheelOutputPolicy<T, C, A> {
    codec: Rc<C>,
    adjustment: Rc<A>,
    marker: PhantomData<fn() -> T>,
}

fn wheel_failure_output<T, StepError, FormatError>(
    rollback: Option<NumericInputEditBatch<T>>,
    failure: NumericInputInteraction<T, StepError, FormatError>,
) -> Option<WidgetOutput>
where
    T: Clone + 'static,
    StepError: 'static,
    FormatError: 'static,
{
    let batch = match rollback {
        Some(cancel) => NumericInputInteractionBatch::from_interactions(&[
            NumericInputInteraction::edit(cancel),
            failure,
        ]),
        None => NumericInputInteractionBatch::from_interactions(std::slice::from_ref(&failure)),
    }?;
    Some(WidgetOutput::typed(batch))
}

impl<T, C, A> WheelOutputPolicy<T> for CompleteWheelOutputPolicy<T, C, A>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    fn wheel(&self, request: WheelRequest<'_, T>) -> Result<T, WheelFailure<T>> {
        let WheelRequest {
            value,
            delta,
            step,
            attempt,
            provenance,
        } = request;
        self.adjustment.wheel(value, delta, step).map_err(|error| {
            let failure: NumericInputInteraction<T, A::Error, C::Error> =
                NumericInputInteraction::wheel_failed(
                    attempt,
                    delta,
                    step,
                    provenance,
                    error,
                    matches!(attempt, NumericWheelAttempt::Update),
                );
            WheelFailure::new(move |rollback| wheel_failure_output(rollback, failure))
        })
    }

    fn format(
        &self,
        request: WheelRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), WheelFailure<T>> {
        let WheelRequest {
            delta,
            step,
            attempt,
            provenance,
            value,
        } = request;
        self.codec.format_editable(value, output).map_err(|error| {
            let failure: NumericInputInteraction<T, A::Error, C::Error> =
                NumericInputInteraction::wheel_format_failed(
                    attempt,
                    delta,
                    step,
                    provenance,
                    error,
                    matches!(attempt, NumericWheelAttempt::Update),
                );
            WheelFailure::new(move |rollback| wheel_failure_output(rollback, failure))
        })
    }
}

pub(super) fn complete_wheel_output_policy<T, C, A>(
    codec: Rc<C>,
    adjustment: Rc<A>,
) -> Rc<dyn WheelOutputPolicy<T>>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    Rc::new(CompleteWheelOutputPolicy {
        codec,
        adjustment,
        marker: PhantomData,
    })
}

pub(super) fn wheel_provenance(sample: WheelSample) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: sample.modifiers(),
        timestamp: sample.timestamp(),
        sequence_range: sample.sequence_range(),
    }
}

/// Convert one exact wheel sample into the policy's signed vertical units.
pub(super) fn wheel_delta(sample: WheelSample) -> Option<f32> {
    if !sample.is_valid() {
        return None;
    }

    let vector = sample.delta().vector();
    if !vector.x.is_finite() || !vector.y.is_finite() || vector.y == 0.0 {
        return None;
    }

    let delta = match sample.delta() {
        WheelDelta::Lines(_) => vector.y,
        WheelDelta::Pixels(_) => vector.y / WHEEL_LINE_EQUIVALENCE_PIXELS,
    };
    delta.is_finite().then_some(delta)
}

pub(super) fn admits_position(bounds: Rect, position: Point) -> bool {
    valid_geometry(bounds, position)
}

pub(super) fn selected_step(modifiers: PointerModifiers) -> NumericStep {
    select_step(modifiers)
}

/// Keep the policy type in this module's dependency surface so an attached
/// policy cannot be accidentally replaced by a legacy vector path.
pub(super) fn is_configured(policy: Option<NumericWheelPolicy>) -> bool {
    policy.is_some()
}
