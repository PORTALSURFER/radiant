use std::rc::Rc;

use super::{
    EditEvent, EditPhase, EditTransaction, InteractionProvenance, InteractionSource, NumericStep,
    NumericStepDirection,
};

/// A typed construction failure for the generic text-first numeric input.
///
/// Construction never invents a fallback draft or range. The application must
/// supply both a codec that can format the initial value and an adjustment that
/// can validate its inverse mapping.
#[derive(Debug, PartialEq)]
pub enum NumericInputConstructionError<CodecError, AdjustmentError> {
    /// The codec could not produce the canonical editable draft.
    CodecFormat {
        /// Codec-provided formatting failure.
        error: CodecError,
    },
    /// The adjustment rejected the initial value's normalized inverse.
    AdjustmentValueToNormalized {
        /// Adjustment-provided inverse-mapping failure.
        error: AdjustmentError,
    },
}

/// One bounded, ordered lifecycle fragment for a generic numeric text edit.
///
/// Accepted fragments are a singleton `Update`, `Commit`, or `Cancel`, or
/// `Begin` followed by one of those phases in the same transaction. Storage is
/// inline and private; the public event slice exposes only the populated
/// prefix. The text-first widget currently emits only `Begin` plus a terminal
/// event; this carrier does not implement semantic keyboard adjustment.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericInputEditBatch<T> {
    events: [EditEvent<T>; 2],
    len: u8,
}

impl<T: Clone> NumericInputEditBatch<T> {
    /// The maximum number of ordered events carried by one batch.
    pub const MAX_EVENTS: usize = 2;

    /// Build a batch from one legal incremental lifecycle fragment.
    ///
    /// A singleton must be `Update`, `Commit`, or `Cancel`. A two-event
    /// fragment must begin with `Begin`, continue with one of those phases,
    /// and share one transaction; any other shape returns `None`.
    pub fn from_events(events: &[EditEvent<T>]) -> Option<Self> {
        match events {
            [event]
                if matches!(
                    event.phase,
                    EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
                ) =>
            {
                Some(Self {
                    events: [event.clone(), event.clone()],
                    len: 1,
                })
            }
            [begin, next]
                if begin.phase == EditPhase::Begin
                    && matches!(
                        next.phase,
                        EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
                    )
                    && begin.transaction == next.transaction =>
            {
                Some(Self {
                    events: [begin.clone(), next.clone()],
                    len: Self::MAX_EVENTS as u8,
                })
            }
            _ => None,
        }
    }

    /// Build the text-first `Begin` plus terminal batch.
    ///
    /// This helper intentionally does not accept an intermediate `Update`;
    /// use [`Self::from_events`] for an incremental fragment.
    pub fn terminal(begin: EditEvent<T>, terminal: EditEvent<T>) -> Option<Self> {
        if begin.phase != EditPhase::Begin || !terminal.phase.is_terminal() {
            return None;
        }
        Self::from_events(&[begin, terminal])
    }

    /// Return the populated ordered events in this batch.
    pub fn events(&self) -> &[EditEvent<T>] {
        &self.events[..usize::from(self.len)]
    }

    /// Return the number of populated events.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether the batch contains no events.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the transaction carried by the populated events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }
}

/// Which phase of a semantic keyboard step was attempted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericStepAttempt {
    /// The first effective step in a keyboard interaction.
    Initial,
    /// A later matching repeat in an existing keyboard interaction.
    Repeat,
}

/// One typed result part for the future semantic numeric keyboard interaction.
///
/// This envelope is a public, UI-local output foundation only. The shipped
/// numeric text widget does not produce these parts, and semantic keyboard
/// stepping remains unimplemented. Failure errors are reference-counted so the
/// envelope can be cloned without requiring either error type to implement
/// `Clone`.
#[derive(Debug, PartialEq)]
pub enum NumericInputInteraction<T, StepError, FormatError> {
    /// A successful, ordered keyboard edit fragment.
    Edit(NumericInputEditBatch<T>),
    /// A typed step-policy failure.
    StepFailed {
        /// Whether the failed attempt was initial or a repeat.
        attempt: NumericStepAttempt,
        /// Direction selected for the attempted step.
        direction: NumericStepDirection,
        /// Step selected for the attempted input.
        step: NumericStep,
        /// Exact keyboard provenance for the attempted input.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<StepError>,
        /// Whether a prior repeat was rolled back before this failure.
        cancelled: bool,
    },
    /// A typed formatting failure.
    FormatFailed {
        /// Whether the failed attempt was initial or a repeat.
        attempt: NumericStepAttempt,
        /// Direction selected for the attempted step.
        direction: NumericStepDirection,
        /// Step selected for the attempted input.
        step: NumericStep,
        /// Exact keyboard provenance for the attempted input.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<FormatError>,
        /// Whether a prior repeat was rolled back before this failure.
        cancelled: bool,
    },
}

impl<T, StepError, FormatError> NumericInputInteraction<T, StepError, FormatError> {
    /// Wrap one successful numeric edit fragment.
    pub fn edit(batch: NumericInputEditBatch<T>) -> Self {
        Self::Edit(batch)
    }

    /// Build a typed step failure while retaining the original error.
    pub fn step_failed(
        attempt: NumericStepAttempt,
        direction: NumericStepDirection,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: StepError,
        cancelled: bool,
    ) -> Self {
        Self::StepFailed {
            attempt,
            direction,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Build a typed formatting failure while retaining the original error.
    pub fn format_failed(
        attempt: NumericStepAttempt,
        direction: NumericStepDirection,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: FormatError,
        cancelled: bool,
    ) -> Self {
        Self::FormatFailed {
            attempt,
            direction,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Return the typed step error by reference when this is `StepFailed`.
    pub fn step_error(&self) -> Option<&StepError> {
        match self {
            Self::StepFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_) | Self::FormatFailed { .. } => None,
        }
    }

    /// Return the typed format error by reference when this is `FormatFailed`.
    pub fn format_error(&self) -> Option<&FormatError> {
        match self {
            Self::FormatFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_) | Self::StepFailed { .. } => None,
        }
    }
}

impl<T: Clone, StepError, FormatError> Clone
    for NumericInputInteraction<T, StepError, FormatError>
{
    fn clone(&self) -> Self {
        match self {
            Self::Edit(batch) => Self::Edit(batch.clone()),
            Self::StepFailed {
                attempt,
                direction,
                step,
                provenance,
                error,
                cancelled,
            } => Self::StepFailed {
                attempt: *attempt,
                direction: *direction,
                step: *step,
                provenance: *provenance,
                error: Rc::clone(error),
                cancelled: *cancelled,
            },
            Self::FormatFailed {
                attempt,
                direction,
                step,
                provenance,
                error,
                cancelled,
            } => Self::FormatFailed {
                attempt: *attempt,
                direction: *direction,
                step: *step,
                provenance: *provenance,
                error: Rc::clone(error),
                cancelled: *cancelled,
            },
        }
    }
}

/// One bounded, ordered envelope of numeric keyboard interaction parts.
///
/// The private inline storage carries at most one successful edit or failure,
/// or a keyboard cancel rollback followed by its repeat failure. It validates
/// output shape and provenance but does not produce, consume, or execute
/// semantic keyboard stepping.
#[derive(Debug, PartialEq)]
pub struct NumericInputInteractionBatch<T, StepError, FormatError> {
    parts: [NumericInputInteraction<T, StepError, FormatError>; 2],
    len: u8,
}

impl<T: Clone, StepError, FormatError> NumericInputInteractionBatch<T, StepError, FormatError> {
    /// Maximum number of ordered interaction parts carried by one envelope.
    pub const MAX_INTERACTIONS: usize = 2;

    /// Wrap one already-validated edit batch in the complete interaction
    /// envelope without changing its events.
    pub(crate) fn from_edit(edit: NumericInputEditBatch<T>) -> Self {
        let part = NumericInputInteraction::edit(edit);
        Self {
            parts: [part.clone(), part],
            len: 1,
        }
    }

    /// Build an envelope from exactly one legal output shape.
    pub fn from_interactions(
        interactions: &[NumericInputInteraction<T, StepError, FormatError>],
    ) -> Option<Self> {
        if !(1..=Self::MAX_INTERACTIONS).contains(&interactions.len())
            || !valid_interactions(interactions)
        {
            return None;
        }

        let first = interactions[0].clone();
        let second = interactions
            .get(1)
            .cloned()
            .unwrap_or_else(|| first.clone());
        Some(Self {
            parts: [first, second],
            len: interactions.len() as u8,
        })
    }

    /// Return the ordered interaction parts in this envelope.
    pub fn parts(&self) -> &[NumericInputInteraction<T, StepError, FormatError>] {
        &self.parts[..usize::from(self.len)]
    }

    /// Return the ordered interaction parts as the envelope's event slice.
    pub fn events(&self) -> &[NumericInputInteraction<T, StepError, FormatError>] {
        self.parts()
    }

    /// Return the number of populated interaction parts.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether the envelope contains no interaction parts.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T: Clone, StepError, FormatError> Clone
    for NumericInputInteractionBatch<T, StepError, FormatError>
{
    fn clone(&self) -> Self {
        Self {
            parts: [self.parts[0].clone(), self.parts[1].clone()],
            len: self.len,
        }
    }
}

fn valid_interactions<T: Clone, StepError, FormatError>(
    interactions: &[NumericInputInteraction<T, StepError, FormatError>],
) -> bool {
    match interactions {
        [NumericInputInteraction::Edit(edit)] => valid_keyboard_edit(edit) || valid_text_edit(edit),
        [
            NumericInputInteraction::StepFailed {
                attempt: NumericStepAttempt::Initial,
                provenance,
                cancelled: false,
                ..
            },
        ]
        | [
            NumericInputInteraction::FormatFailed {
                attempt: NumericStepAttempt::Initial,
                provenance,
                cancelled: false,
                ..
            },
        ] => is_keyboard_provenance(*provenance),
        [NumericInputInteraction::Edit(edit), failure] => {
            let [cancel] = edit.events() else {
                return false;
            };
            if cancel.phase != EditPhase::Cancel || !is_keyboard_provenance(cancel.provenance) {
                return false;
            }
            match failure {
                NumericInputInteraction::StepFailed {
                    attempt: NumericStepAttempt::Repeat,
                    provenance,
                    cancelled: true,
                    ..
                }
                | NumericInputInteraction::FormatFailed {
                    attempt: NumericStepAttempt::Repeat,
                    provenance,
                    cancelled: true,
                    ..
                } => is_keyboard_provenance(*provenance) && *provenance == cancel.provenance,
                NumericInputInteraction::Edit(_)
                | NumericInputInteraction::StepFailed { .. }
                | NumericInputInteraction::FormatFailed { .. } => false,
            }
        }
        _ => false,
    }
}

fn valid_keyboard_edit<T: Clone>(edit: &NumericInputEditBatch<T>) -> bool {
    match edit.events() {
        [event]
            if matches!(
                event.phase,
                EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
            ) =>
        {
            is_keyboard_provenance(event.provenance)
        }
        [begin, update] if begin.phase == EditPhase::Begin && update.phase == EditPhase::Update => {
            is_keyboard_provenance(begin.provenance) && is_keyboard_provenance(update.provenance)
        }
        _ => false,
    }
}

fn valid_text_edit<T: Clone>(edit: &NumericInputEditBatch<T>) -> bool {
    match edit.events() {
        [begin, terminal]
            if begin.phase == EditPhase::Begin
                && matches!(terminal.phase, EditPhase::Commit | EditPhase::Cancel) =>
        {
            is_keyboard_provenance(begin.provenance) && is_keyboard_provenance(terminal.provenance)
        }
        _ => false,
    }
}

fn is_keyboard_provenance(provenance: InteractionProvenance) -> bool {
    provenance.source() == InteractionSource::Keyboard
}
