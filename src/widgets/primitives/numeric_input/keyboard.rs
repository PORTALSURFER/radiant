use std::rc::Rc;

use crate::{
    gui::input::InputTimestamp,
    widgets::{
        EditEvent, InteractionProvenance, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputEditBatch, NumericInputInteraction, NumericInputInteractionBatch, NumericStep,
        NumericStepAttempt, NumericStepDirection, WidgetKey, WidgetOutput,
    },
};

/// The retained state for one admitted semantic keyboard adjustment.
#[derive(Clone)]
pub(super) struct KeyboardAdjustmentState<T> {
    pub(super) session: NumericEditSession<T>,
    pub(super) key: WidgetKey,
    pub(super) start_text: String,
    pub(super) start_caret: usize,
    pub(super) start_selection_anchor: usize,
}

impl<T: Clone> KeyboardAdjustmentState<T> {
    pub(super) fn new(
        value: T,
        draft: String,
        key: WidgetKey,
        timestamp: Option<InputTimestamp>,
        start_caret: usize,
        start_selection_anchor: usize,
    ) -> Self {
        Self {
            session: NumericEditSession::begin(
                value,
                draft.clone(),
                InteractionProvenance::Keyboard { timestamp },
            ),
            key,
            start_text: draft,
            start_caret,
            start_selection_anchor,
        }
    }

    pub(super) fn begin_update(
        &self,
        value: T,
        timestamp: Option<InputTimestamp>,
    ) -> Option<EditEvent<T>> {
        self.session
            .begin_event()
            .clone()
            .update(value, InteractionProvenance::Keyboard { timestamp })
    }

    pub(super) fn commit(
        self,
        value: T,
        timestamp: Option<InputTimestamp>,
    ) -> Result<EditEvent<T>, Box<Self>> {
        let Self {
            session,
            key,
            start_text,
            start_caret,
            start_selection_anchor,
        } = self;
        match session.commit(value, InteractionProvenance::Keyboard { timestamp }) {
            Ok(event) => Ok(event),
            Err(session) => Err(Box::new(Self {
                session,
                key,
                start_text,
                start_caret,
                start_selection_anchor,
            })),
        }
    }

    pub(super) fn cancel(
        self,
        timestamp: Option<InputTimestamp>,
    ) -> Result<EditEvent<T>, Box<Self>> {
        let Self {
            session,
            key,
            start_text,
            start_caret,
            start_selection_anchor,
        } = self;
        match session.cancel(InteractionProvenance::Keyboard { timestamp }) {
            Ok(event) => Ok(event),
            Err(session) => Err(Box::new(Self {
                session,
                key,
                start_text,
                start_caret,
                start_selection_anchor,
            })),
        }
    }
}

pub(super) struct KeyboardAdjustmentRequest<'a, T> {
    pub(super) value: &'a T,
    pub(super) direction: NumericStepDirection,
    pub(super) step: NumericStep,
    pub(super) attempt: NumericStepAttempt,
    pub(super) provenance: InteractionProvenance,
    pub(super) rollback: Option<NumericInputEditBatch<T>>,
}

/// UI-local policy boundary for keyboard adjustment and its typed failures.
///
/// The complete implementation is installed only after the existing complete
/// output encoder has established its associated-error lifetime contract. This
/// keeps the compatibility widget surface free of new error bounds.
pub(super) trait KeyboardAdjustmentPolicy<T> {
    fn step(&self, request: KeyboardAdjustmentRequest<'_, T>) -> Result<T, Option<WidgetOutput>>;

    fn format(
        &self,
        request: KeyboardAdjustmentRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), Option<WidgetOutput>>;
}

struct NoKeyboardAdjustmentPolicy;

impl<T> KeyboardAdjustmentPolicy<T> for NoKeyboardAdjustmentPolicy {
    fn step(&self, _request: KeyboardAdjustmentRequest<'_, T>) -> Result<T, Option<WidgetOutput>> {
        Err(None)
    }

    fn format(
        &self,
        _request: KeyboardAdjustmentRequest<'_, T>,
        _output: &mut String,
    ) -> Result<(), Option<WidgetOutput>> {
        Err(None)
    }
}

pub(super) fn no_keyboard_adjustment_policy<T>() -> Rc<dyn KeyboardAdjustmentPolicy<T>> {
    Rc::new(NoKeyboardAdjustmentPolicy)
}

struct CompleteKeyboardAdjustmentPolicy<T, C, A> {
    codec: Rc<C>,
    adjustment: Rc<A>,
    marker: std::marker::PhantomData<fn() -> T>,
}

impl<T, C, A> CompleteKeyboardAdjustmentPolicy<T, C, A> {
    fn failure(
        rollback: Option<NumericInputEditBatch<T>>,
        failure: NumericInputInteraction<T, A::Error, C::Error>,
    ) -> Option<WidgetOutput>
    where
        T: Clone + 'static,
        C: NumericCodec<T> + 'static,
        A: NumericAdjustment<T> + 'static,
        A::Error: 'static,
        C::Error: 'static,
    {
        let batch = match rollback {
            Some(cancel) => {
                let cancel = NumericInputInteraction::edit(cancel);
                NumericInputInteractionBatch::from_interactions(&[cancel, failure])
            }
            None => NumericInputInteractionBatch::from_interactions(std::slice::from_ref(&failure)),
        }?;
        Some(WidgetOutput::typed(batch))
    }
}

impl<T, C, A> KeyboardAdjustmentPolicy<T> for CompleteKeyboardAdjustmentPolicy<T, C, A>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    fn step(&self, request: KeyboardAdjustmentRequest<'_, T>) -> Result<T, Option<WidgetOutput>> {
        let KeyboardAdjustmentRequest {
            value,
            direction,
            step,
            attempt,
            provenance,
            rollback,
        } = request;
        self.adjustment
            .step(value, direction, step)
            .map_err(|error| {
                Self::failure(
                    rollback,
                    NumericInputInteraction::step_failed(
                        attempt,
                        direction,
                        step,
                        provenance,
                        error,
                        matches!(attempt, NumericStepAttempt::Repeat),
                    ),
                )
            })
    }

    fn format(
        &self,
        request: KeyboardAdjustmentRequest<'_, T>,
        output: &mut String,
    ) -> Result<(), Option<WidgetOutput>> {
        let KeyboardAdjustmentRequest {
            value,
            direction,
            step,
            attempt,
            provenance,
            rollback,
        } = request;
        self.codec.format_editable(value, output).map_err(|error| {
            Self::failure(
                rollback,
                NumericInputInteraction::format_failed(
                    attempt,
                    direction,
                    step,
                    provenance,
                    error,
                    matches!(attempt, NumericStepAttempt::Repeat),
                ),
            )
        })
    }
}

pub(super) fn complete_keyboard_adjustment_policy<T, C, A>(
    codec: Rc<C>,
    adjustment: Rc<A>,
) -> Rc<dyn KeyboardAdjustmentPolicy<T>>
where
    T: Clone + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
    A::Error: 'static,
    C::Error: 'static,
{
    Rc::new(CompleteKeyboardAdjustmentPolicy {
        codec,
        adjustment,
        marker: std::marker::PhantomData,
    })
}

pub(super) const fn direction_for_key(key: WidgetKey) -> Option<NumericStepDirection> {
    match key {
        WidgetKey::ArrowUp => Some(NumericStepDirection::Increase),
        WidgetKey::ArrowDown => Some(NumericStepDirection::Decrease),
        _ => None,
    }
}

pub(super) const fn is_adjustment_key(key: WidgetKey) -> bool {
    direction_for_key(key).is_some()
}
