use std::rc::Rc;

use super::{
    EditEvent, EditPhase, EditTransaction, InteractionProvenance, InteractionSource, NumericStep,
    NumericStepDirection,
    input::{PointerButton, PointerModifiers},
    numeric_step_modifiers::KeyboardModifier,
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
/// prefix. The text-first widget emits terminal `Begin` plus a terminal event,
/// while complete-mode keyboard adjustment also consumes incremental
/// `Begin` plus `Update`, singleton `Update`, `Commit`, and `Cancel` shapes.
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

/// The attempt boundary for one complete-mode pointer scrub sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericScrubAttempt {
    /// The first effective candidate in an admitted pointer scrub.
    Initial,
    /// A later effective candidate in an active pointer scrub.
    Update,
}

/// The explicit activation gesture for complete-mode numeric pointer scrubbing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericScrubActivation {
    /// A primary-button pointer press carrying one semantic modifier.
    PrimaryButtonHorizontalDrag {
        /// The normalized modifier latched when the press is admitted.
        modifier: KeyboardModifier,
    },
}

impl Default for NumericScrubActivation {
    fn default() -> Self {
        Self::PrimaryButtonHorizontalDrag {
            modifier: KeyboardModifier::Alt,
        }
    }
}

/// Backend-neutral, explicit policy for complete-mode numeric pointer scrubbing.
///
/// The policy enables no behavior until attached with
/// [`NumericInputBuilder::scrub_policy`](crate::application::NumericInputBuilder::scrub_policy).
/// Once attached, each move selects Base, Fine, or Coarse from its own
/// normalized pointer modifiers; Shift selects Fine and the normalized
/// platform-command modifier selects Coarse, with Fine taking precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NumericScrubPolicy {
    activation: NumericScrubActivation,
}

impl NumericScrubPolicy {
    /// Construct a policy with an explicit activation gesture.
    pub const fn new(activation: NumericScrubActivation) -> Self {
        Self { activation }
    }

    /// Return the activation gesture carried by this policy.
    pub const fn activation(&self) -> NumericScrubActivation {
        self.activation
    }

    pub(crate) const fn activation_modifier(self) -> KeyboardModifier {
        match self.activation {
            NumericScrubActivation::PrimaryButtonHorizontalDrag { modifier } => modifier,
        }
    }

    pub(crate) fn admits(self, button: PointerButton, modifiers: PointerModifiers) -> bool {
        if button != PointerButton::Primary {
            return false;
        }

        match self.activation {
            NumericScrubActivation::PrimaryButtonHorizontalDrag { modifier } => {
                modifier_is_held(modifier, modifiers)
            }
        }
    }
}

impl Default for NumericScrubPolicy {
    fn default() -> Self {
        Self::new(NumericScrubActivation::default())
    }
}

const fn modifier_is_held(modifier: KeyboardModifier, modifiers: PointerModifiers) -> bool {
    match modifier {
        KeyboardModifier::Shift => modifiers.shift,
        KeyboardModifier::Command | KeyboardModifier::Control => modifiers.command,
        KeyboardModifier::Alt => modifiers.alt,
    }
}

/// One typed result part for complete-mode numeric text, keyboard, or pointer
/// interaction.
///
/// The complete numeric widget produces successful edits and typed keyboard or
/// pointer failures when the corresponding explicit policy is attached.
/// Failure errors are reference-counted so the envelope can be cloned without
/// requiring either error type to implement `Clone`.
#[derive(Debug, PartialEq)]
pub enum NumericInputInteraction<T, StepError, FormatError> {
    /// A successful, ordered edit fragment.
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
    /// A typed pointer-scrub adjustment failure.
    ScrubFailed {
        /// Whether the failed attempt was initial or already active.
        attempt: NumericScrubAttempt,
        /// The normalized horizontal delta supplied to the adjustment policy.
        normalized_delta: f32,
        /// Step selected for the attempted pointer sample.
        step: NumericStep,
        /// Exact pointer provenance for the attempted sample.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<StepError>,
        /// Whether an active scrub was rolled back before this failure.
        cancelled: bool,
    },
    /// A typed pointer-scrub formatting failure.
    PointerFormatFailed {
        /// Whether the failed attempt was initial or already active.
        attempt: NumericScrubAttempt,
        /// The normalized horizontal delta for the candidate being formatted.
        normalized_delta: f32,
        /// Step selected for the attempted pointer sample.
        step: NumericStep,
        /// Exact pointer provenance for the attempted sample.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<FormatError>,
        /// Whether an active scrub was rolled back before this failure.
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

    /// Build a typed pointer-scrub adjustment failure.
    pub fn scrub_failed(
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: StepError,
        cancelled: bool,
    ) -> Self {
        Self::ScrubFailed {
            attempt,
            normalized_delta,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Alias for [`Self::scrub_failed`] that names the pointer boundary.
    pub fn pointer_scrub_failed(
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: StepError,
        cancelled: bool,
    ) -> Self {
        Self::scrub_failed(
            attempt,
            normalized_delta,
            step,
            provenance,
            error,
            cancelled,
        )
    }

    /// Build a typed pointer-scrub formatting failure.
    pub fn pointer_format_failed(
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: FormatError,
        cancelled: bool,
    ) -> Self {
        Self::PointerFormatFailed {
            attempt,
            normalized_delta,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Alias for [`Self::pointer_format_failed`] using scrub terminology.
    pub fn scrub_format_failed(
        attempt: NumericScrubAttempt,
        normalized_delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: FormatError,
        cancelled: bool,
    ) -> Self {
        Self::pointer_format_failed(
            attempt,
            normalized_delta,
            step,
            provenance,
            error,
            cancelled,
        )
    }

    /// Return the typed step error by reference when this is `StepFailed`.
    pub fn step_error(&self) -> Option<&StepError> {
        match self {
            Self::StepFailed { error, .. } | Self::ScrubFailed { error, .. } => {
                Some(error.as_ref())
            }
            Self::Edit(_) | Self::FormatFailed { .. } | Self::PointerFormatFailed { .. } => None,
        }
    }

    /// Return the typed pointer-scrub adjustment error by reference.
    pub fn scrub_error(&self) -> Option<&StepError> {
        match self {
            Self::ScrubFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::PointerFormatFailed { .. } => None,
        }
    }

    /// Return the typed format error by reference when this is `FormatFailed`.
    pub fn format_error(&self) -> Option<&FormatError> {
        match self {
            Self::FormatFailed { error, .. } | Self::PointerFormatFailed { error, .. } => {
                Some(error.as_ref())
            }
            Self::Edit(_) | Self::StepFailed { .. } | Self::ScrubFailed { .. } => None,
        }
    }

    /// Return the typed pointer-scrub formatting error by reference.
    pub fn pointer_format_error(&self) -> Option<&FormatError> {
        match self {
            Self::PointerFormatFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::ScrubFailed { .. } => None,
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
            Self::ScrubFailed {
                attempt,
                normalized_delta,
                step,
                provenance,
                error,
                cancelled,
            } => Self::ScrubFailed {
                attempt: *attempt,
                normalized_delta: *normalized_delta,
                step: *step,
                provenance: *provenance,
                error: Rc::clone(error),
                cancelled: *cancelled,
            },
            Self::PointerFormatFailed {
                attempt,
                normalized_delta,
                step,
                provenance,
                error,
                cancelled,
            } => Self::PointerFormatFailed {
                attempt: *attempt,
                normalized_delta: *normalized_delta,
                step: *step,
                provenance: *provenance,
                error: Rc::clone(error),
                cancelled: *cancelled,
            },
        }
    }
}

/// One bounded, ordered envelope of complete numeric interaction parts.
///
/// The private inline storage carries at most one successful edit or failure,
/// or a cancellation rollback followed by its update failure. It validates
/// output shape and provenance for the complete-mode numeric consumers.
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
        [NumericInputInteraction::Edit(edit)] => {
            valid_keyboard_edit(edit) || valid_pointer_edit(edit) || valid_text_edit(edit)
        }
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
        [
            NumericInputInteraction::ScrubFailed {
                attempt: NumericScrubAttempt::Initial,
                provenance,
                cancelled: false,
                ..
            },
        ]
        | [
            NumericInputInteraction::PointerFormatFailed {
                attempt: NumericScrubAttempt::Initial,
                provenance,
                cancelled: false,
                ..
            },
        ] => is_pointer_provenance(*provenance),
        [NumericInputInteraction::Edit(edit), failure] => {
            let [cancel] = edit.events() else {
                return false;
            };
            if cancel.phase != EditPhase::Cancel {
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
                } => {
                    is_keyboard_provenance(cancel.provenance)
                        && is_keyboard_provenance(*provenance)
                        && *provenance == cancel.provenance
                }
                NumericInputInteraction::ScrubFailed {
                    attempt: NumericScrubAttempt::Update,
                    provenance,
                    cancelled: true,
                    ..
                }
                | NumericInputInteraction::PointerFormatFailed {
                    attempt: NumericScrubAttempt::Update,
                    provenance,
                    cancelled: true,
                    ..
                } => {
                    is_pointer_provenance(cancel.provenance)
                        && is_pointer_provenance(*provenance)
                        && *provenance == cancel.provenance
                }
                NumericInputInteraction::Edit(_)
                | NumericInputInteraction::StepFailed { .. }
                | NumericInputInteraction::FormatFailed { .. }
                | NumericInputInteraction::ScrubFailed { .. }
                | NumericInputInteraction::PointerFormatFailed { .. } => false,
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

fn valid_pointer_edit<T: Clone>(edit: &NumericInputEditBatch<T>) -> bool {
    match edit.events() {
        [event]
            if matches!(
                event.phase,
                EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
            ) =>
        {
            is_pointer_provenance(event.provenance)
        }
        [begin, update] if begin.phase == EditPhase::Begin && update.phase == EditPhase::Update => {
            is_pointer_provenance(begin.provenance) && is_pointer_provenance(update.provenance)
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

fn is_pointer_provenance(provenance: InteractionProvenance) -> bool {
    provenance.source() == InteractionSource::Pointer
}
