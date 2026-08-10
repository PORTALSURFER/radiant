//! Complete-mode numeric pointer-scrub state and typed output policy.

use std::{marker::PhantomData, rc::Rc};

use crate::{
    gui::{
        input::{InputSequenceRange, InputTimestamp},
        types::{Point, Rect},
    },
    widgets::{
        InteractionProvenance, NumericAdjustment, NumericCodec, NumericInputEditBatch,
        NumericInputInteraction, NumericInputInteractionBatch, NumericScrubAttempt,
        NumericScrubPolicy, NumericStep, PointerModifiers, WidgetOutput,
    },
};

/// Retained state for one admitted numeric pointer scrub.
#[derive(Clone)]
pub(super) struct PointerScrubState<T> {
    pub(super) start_value: T,
    pub(super) start_text: String,
    pub(super) start_caret: usize,
    pub(super) start_selection_anchor: usize,
    pub(super) press_position: Point,
    pub(super) bounds: Rect,
    pub(super) press_provenance: InteractionProvenance,
    pub(super) anchor_position: Point,
    pub(super) anchor_value: T,
    pub(super) step: NumericStep,
    pub(super) session: Option<crate::widgets::NumericEditSession<T>>,
    pub(super) activation_modifier: crate::widgets::KeyboardModifier,
}

impl<T: Clone> PointerScrubState<T> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        value: T,
        text: String,
        caret: usize,
        selection_anchor: usize,
        position: Point,
        bounds: Rect,
        press_provenance: InteractionProvenance,
        policy: NumericScrubPolicy,
        modifiers: PointerModifiers,
    ) -> Self {
        let activation_modifier = policy.activation_modifier();
        let step = select_step(without_activation_modifier(modifiers, activation_modifier));
        Self {
            start_value: value.clone(),
            start_text: text,
            start_caret: caret,
            start_selection_anchor: selection_anchor,
            press_position: position,
            bounds,
            press_provenance,
            anchor_position: position,
            anchor_value: value,
            step,
            session: None,
            activation_modifier,
        }
    }

    pub(super) const fn is_active(&self) -> bool {
        self.session.is_some()
    }
}

/// A failure which delays typed output construction until the handler knows
/// whether an active transaction needs a rollback part.
pub(super) struct PointerScrubFailure<T> {
    emit: Box<dyn FnOnce(Option<NumericInputEditBatch<T>>) -> Option<WidgetOutput>>,
}

impl<T> PointerScrubFailure<T> {
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

/// One policy call made by a non-no-op pointer move.
pub(super) struct PointerScrubRequest<'a, T> {
    pub(super) value: &'a T,
    pub(super) normalized_delta: f32,
    pub(super) step: NumericStep,
    pub(super) attempt: NumericScrubAttempt,
    pub(super) provenance: InteractionProvenance,
}

/// Complete-mode typed pointer policy boundary.
pub(super) trait PointerScrubOutputPolicy<T> {
    fn scrub(&self, request: PointerScrubRequest<'_, T>) -> Result<T, PointerScrubFailure<T>>;

    fn format(
        &self,
        request: PointerScrubRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), PointerScrubFailure<T>>;
}

struct CompletePointerScrubOutputPolicy<T, C, A> {
    codec: Rc<C>,
    adjustment: Rc<A>,
    marker: PhantomData<fn() -> T>,
}

fn pointer_failure_output<T, StepError, FormatError>(
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

impl<T, C, A> PointerScrubOutputPolicy<T> for CompletePointerScrubOutputPolicy<T, C, A>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    fn scrub(&self, request: PointerScrubRequest<'_, T>) -> Result<T, PointerScrubFailure<T>> {
        let PointerScrubRequest {
            value,
            normalized_delta,
            step,
            attempt,
            provenance,
        } = request;
        self.adjustment
            .scrub(value, normalized_delta, step)
            .map_err(|error| {
                let failure: NumericInputInteraction<T, A::Error, C::Error> =
                    NumericInputInteraction::scrub_failed(
                        attempt,
                        normalized_delta,
                        step,
                        provenance,
                        error,
                        matches!(attempt, NumericScrubAttempt::Update),
                    );
                PointerScrubFailure::new(move |rollback| pointer_failure_output(rollback, failure))
            })
    }

    fn format(
        &self,
        request: PointerScrubRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), PointerScrubFailure<T>> {
        let PointerScrubRequest {
            value,
            normalized_delta,
            step,
            attempt,
            provenance,
        } = request;
        self.codec.format_editable(value, output).map_err(|error| {
            let failure: NumericInputInteraction<T, A::Error, C::Error> =
                NumericInputInteraction::pointer_format_failed(
                    attempt,
                    normalized_delta,
                    step,
                    provenance,
                    error,
                    matches!(attempt, NumericScrubAttempt::Update),
                );
            PointerScrubFailure::new(move |rollback| pointer_failure_output(rollback, failure))
        })
    }
}

pub(super) fn complete_pointer_scrub_output_policy<T, C, A>(
    codec: Rc<C>,
    adjustment: Rc<A>,
) -> Rc<dyn PointerScrubOutputPolicy<T>>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    Rc::new(CompletePointerScrubOutputPolicy {
        codec,
        adjustment,
        marker: PhantomData,
    })
}

pub(super) fn pointer_provenance(
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

pub(super) fn without_activation_modifier(
    mut modifiers: PointerModifiers,
    activation_modifier: crate::widgets::KeyboardModifier,
) -> PointerModifiers {
    match activation_modifier {
        crate::widgets::KeyboardModifier::Shift => modifiers.shift = false,
        crate::widgets::KeyboardModifier::Command | crate::widgets::KeyboardModifier::Control => {
            modifiers.command = false
        }
        crate::widgets::KeyboardModifier::Alt => modifiers.alt = false,
    }
    modifiers
}

pub(super) const fn select_step(modifiers: PointerModifiers) -> NumericStep {
    if modifiers.shift {
        NumericStep::Fine
    } else if modifiers.command {
        NumericStep::Coarse
    } else {
        NumericStep::Base
    }
}

pub(super) fn valid_geometry(bounds: Rect, position: Point) -> bool {
    bounds.is_finite() && bounds.width() > 0.0 && position.is_finite() && bounds.contains(position)
}

pub(super) fn normalized_delta(bounds: Rect, anchor: Point, position: Point) -> Option<f32> {
    let delta = (position.x - anchor.x) / bounds.width();
    delta.is_finite().then_some(delta)
}
