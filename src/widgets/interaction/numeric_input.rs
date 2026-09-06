use std::{marker::PhantomData, rc::Rc};

use super::numeric_ownership::NumericInteractionOwner;

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
/// Accepted fragments are a singleton `Update`, `Commit`, or `Cancel`, a
/// `Begin` followed by one of those phases in the same transaction, or the
/// complete-mode wheel transaction `Begin`, `Update`, `Commit`. Storage is
/// inline and private; the public event slice exposes only the populated
/// prefix. The text-first widget emits terminal `Begin` plus a terminal event,
/// while complete-mode keyboard adjustment also consumes incremental
/// `Begin` plus `Update`, singleton `Update`, `Commit`, and `Cancel` shapes.
#[derive(Clone, Debug, PartialEq)]
pub struct NumericInputEditBatch<T> {
    events: [EditEvent<T>; 3],
    len: u8,
}

impl<T: Clone> NumericInputEditBatch<T> {
    /// The maximum number of ordered events carried by one batch.
    pub const MAX_EVENTS: usize = 3;

    /// Build a batch from one legal incremental lifecycle fragment.
    ///
    /// A singleton must be `Update`, `Commit`, or `Cancel`. A two-event
    /// fragment must begin with `Begin`, continue with one of those phases,
    /// and share one transaction. The only three-event fragment is
    /// same-transaction `Begin`, `Update`, `Commit`; any other shape returns
    /// `None`.
    pub fn from_events(events: &[EditEvent<T>]) -> Option<Self> {
        match events {
            [event]
                if matches!(
                    event.phase,
                    EditPhase::Update | EditPhase::Commit | EditPhase::Cancel
                ) =>
            {
                Some(Self {
                    events: [event.clone(), event.clone(), event.clone()],
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
                    events: [begin.clone(), next.clone(), next.clone()],
                    len: 2,
                })
            }
            [begin, update, commit]
                if begin.phase == EditPhase::Begin
                    && update.phase == EditPhase::Update
                    && commit.phase == EditPhase::Commit
                    && begin.transaction == update.transaction
                    && begin.transaction == commit.transaction =>
            {
                Some(Self {
                    events: [begin.clone(), update.clone(), commit.clone()],
                    len: 3,
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

/// The attempt boundary for one complete-mode numeric wheel sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericWheelAttempt {
    /// The first effective candidate in an admitted wheel sequence or atomic gesture.
    Initial,
    /// A later effective candidate in an active wheel sequence.
    Update,
}

/// Backend-neutral, explicit policy for complete-mode numeric wheel adjustment.
///
/// The policy enables no behavior until attached with
/// [`NumericInputBuilder::wheel_policy`](crate::application::NumericInputBuilder::wheel_policy).
/// Wheel unit conversion and sequence ownership remain fixed by the generic
/// NumericInput contract; [`NumericAdjustment::wheel`](crate::widgets::NumericAdjustment::wheel)
/// owns domain-specific sensitivity and mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NumericWheelPolicy {
    marker: PhantomData<()>,
}

impl NumericWheelPolicy {
    /// Construct the fixed backend-neutral wheel policy.
    pub const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
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
/// The complete numeric widget produces successful edits and typed keyboard,
/// pointer, or wheel failures when the corresponding explicit policy is
/// attached.
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
    /// A typed numeric wheel adjustment failure.
    WheelFailed {
        /// Whether the failed attempt was initial or already active.
        attempt: NumericWheelAttempt,
        /// Signed vertical delta passed to the adjustment policy.
        delta: f32,
        /// Step selected for the attempted wheel sample.
        step: NumericStep,
        /// Exact pointer provenance for the attempted sample.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<StepError>,
        /// Whether an active wheel edit was rolled back before this failure.
        cancelled: bool,
    },
    /// A typed numeric wheel formatting failure.
    WheelFormatFailed {
        /// Whether the failed attempt was initial or already active.
        attempt: NumericWheelAttempt,
        /// Signed vertical delta for the candidate being formatted.
        delta: f32,
        /// Step selected for the attempted wheel sample.
        step: NumericStep,
        /// Exact pointer provenance for the attempted sample.
        provenance: InteractionProvenance,
        /// UI-local typed failure storage.
        error: Rc<FormatError>,
        /// Whether an active wheel edit was rolled back before this failure.
        cancelled: bool,
    },
}

/// A discrete, backend-neutral action for the generic numeric input.
///
/// This is the widget-local policy vocabulary. Runtime target resolution,
/// focus admission, stale-target classification, and native action mapping are
/// separate contracts at the runtime and platform boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NumericAccessibilityAction {
    /// Apply one ordinary increase step.
    Increment,
    /// Apply one ordinary decrease step.
    Decrement,
    /// Replace the value with one complete editable text payload.
    SetValueText(String),
}

/// Deterministic rejection reason produced by the widget-local accessibility
/// consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAccessibilityRejectedReason {
    /// The complete interaction policy is not installed for this widget.
    UnsupportedAction,
    /// The widget is disabled.
    Disabled,
    /// The widget is read-only.
    ReadOnly,
    /// The runtime has not admitted the target as focused and editable.
    NotFocusable,
    /// The focused widget vetoed the ordinary focus transition.
    FocusDenied,
    /// The complete text payload is an accepted prefix but not a value.
    Incomplete,
    /// The complete text payload does not match the codec grammar.
    Invalid,
    /// The complete text payload is outside the codec domain.
    OutOfRange,
}

/// Public owner vocabulary for an accessibility action that must not interrupt
/// an incumbent numeric interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NumericAccessibilityBlockOwner {
    /// A recognized widget or container gesture owns interaction capture.
    GestureCapture,
    /// A normal text edit owns the numeric input.
    TextEdit,
    /// An IME composition owns the numeric input.
    ImeComposition,
    /// A keyboard adjustment owns the numeric input.
    KeyboardAdjustment,
    /// A pointer scrub owns the numeric input.
    PointerScrub,
    /// A wheel sequence owns the numeric input.
    WheelSequence,
    /// Another accessibility action owns the numeric input.
    AccessibilityEdit,
}

impl From<NumericInteractionOwner> for NumericAccessibilityBlockOwner {
    fn from(owner: NumericInteractionOwner) -> Self {
        match owner {
            NumericInteractionOwner::TextEdit => Self::TextEdit,
            NumericInteractionOwner::ImeComposition => Self::ImeComposition,
            NumericInteractionOwner::KeyboardAdjustment => Self::KeyboardAdjustment,
            NumericInteractionOwner::PointerScrub => Self::PointerScrub,
            NumericInteractionOwner::WheelSequence => Self::WheelSequence,
            NumericInteractionOwner::AccessibilityEdit => Self::AccessibilityEdit,
        }
    }
}

/// Result of the generic numeric widget's local accessibility policy.
///
/// Stale/removed/unmaterialized target outcomes and pre-focus admission are
/// owned by the runtime dispatch boundary and therefore do not appear here. A
/// successful changed action is one bounded `Begin`/`Update`/`Commit` batch;
/// all other outcomes leave the exact widget state unchanged. The runtime
/// carries this local result through its type-erased output boundary after
/// admission.
#[derive(Debug, PartialEq)]
pub enum NumericAccessibilityOutcome<T, AdjustmentError, FormatError> {
    /// One accepted changed action with its complete atomic edit lifecycle.
    Edit(NumericInputEditBatch<T>),
    /// An accepted policy action produced the current value exactly.
    NoChange {
        /// The action that was evaluated.
        action: NumericAccessibilityAction,
    },
    /// The local consumer rejected the action without editing.
    Rejected {
        /// The action that was rejected.
        action: NumericAccessibilityAction,
        /// Why the local consumer rejected it.
        reason: NumericAccessibilityRejectedReason,
    },
    /// An incumbent interaction owns the input and remains untouched.
    Blocked {
        /// The incumbent owner that prevented admission.
        owner: NumericAccessibilityBlockOwner,
    },
    /// The adjustment policy could not produce a candidate value.
    AdjustmentFailed {
        /// The action whose adjustment failed.
        action: NumericAccessibilityAction,
        /// The policy-provided failure.
        error: Rc<AdjustmentError>,
    },
    /// The codec could not format a changed candidate.
    FormatFailed {
        /// The action whose formatting failed.
        action: NumericAccessibilityAction,
        /// The codec-provided failure.
        error: Rc<FormatError>,
    },
}

impl<T: Clone, AdjustmentError, FormatError> Clone
    for NumericAccessibilityOutcome<T, AdjustmentError, FormatError>
{
    fn clone(&self) -> Self {
        match self {
            Self::Edit(edit) => Self::Edit(edit.clone()),
            Self::NoChange { action } => Self::NoChange {
                action: action.clone(),
            },
            Self::Rejected { action, reason } => Self::Rejected {
                action: action.clone(),
                reason: *reason,
            },
            Self::Blocked { owner } => Self::Blocked { owner: *owner },
            Self::AdjustmentFailed { action, error } => Self::AdjustmentFailed {
                action: action.clone(),
                error: Rc::clone(error),
            },
            Self::FormatFailed { action, error } => Self::FormatFailed {
                action: action.clone(),
                error: Rc::clone(error),
            },
        }
    }
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

    /// Build a typed numeric wheel adjustment failure.
    pub fn wheel_failed(
        attempt: NumericWheelAttempt,
        delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: StepError,
        cancelled: bool,
    ) -> Self {
        Self::WheelFailed {
            attempt,
            delta,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Build a typed numeric wheel formatting failure.
    pub fn wheel_format_failed(
        attempt: NumericWheelAttempt,
        delta: f32,
        step: NumericStep,
        provenance: InteractionProvenance,
        error: FormatError,
        cancelled: bool,
    ) -> Self {
        Self::WheelFormatFailed {
            attempt,
            delta,
            step,
            provenance,
            error: Rc::new(error),
            cancelled,
        }
    }

    /// Return the typed step error by reference when this is `StepFailed`.
    pub fn step_error(&self) -> Option<&StepError> {
        match self {
            Self::StepFailed { error, .. }
            | Self::ScrubFailed { error, .. }
            | Self::WheelFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::FormatFailed { .. }
            | Self::PointerFormatFailed { .. }
            | Self::WheelFormatFailed { .. } => None,
        }
    }

    /// Return the typed pointer-scrub adjustment error by reference.
    pub fn scrub_error(&self) -> Option<&StepError> {
        match self {
            Self::ScrubFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::PointerFormatFailed { .. }
            | Self::WheelFailed { .. }
            | Self::WheelFormatFailed { .. } => None,
        }
    }

    /// Return the typed format error by reference when this is `FormatFailed`.
    pub fn format_error(&self) -> Option<&FormatError> {
        match self {
            Self::FormatFailed { error, .. }
            | Self::PointerFormatFailed { error, .. }
            | Self::WheelFormatFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::ScrubFailed { .. }
            | Self::WheelFailed { .. } => None,
        }
    }

    /// Return the typed pointer-scrub formatting error by reference.
    pub fn pointer_format_error(&self) -> Option<&FormatError> {
        match self {
            Self::PointerFormatFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::ScrubFailed { .. }
            | Self::WheelFailed { .. }
            | Self::WheelFormatFailed { .. } => None,
        }
    }

    /// Return the typed numeric wheel adjustment error.
    pub fn wheel_error(&self) -> Option<&StepError> {
        match self {
            Self::WheelFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::ScrubFailed { .. }
            | Self::PointerFormatFailed { .. }
            | Self::WheelFormatFailed { .. } => None,
        }
    }

    /// Return the typed numeric wheel formatting error.
    pub fn wheel_format_error(&self) -> Option<&FormatError> {
        match self {
            Self::WheelFormatFailed { error, .. } => Some(error.as_ref()),
            Self::Edit(_)
            | Self::StepFailed { .. }
            | Self::FormatFailed { .. }
            | Self::ScrubFailed { .. }
            | Self::PointerFormatFailed { .. }
            | Self::WheelFailed { .. } => None,
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
            Self::WheelFailed {
                attempt,
                delta,
                step,
                provenance,
                error,
                cancelled,
            } => Self::WheelFailed {
                attempt: *attempt,
                delta: *delta,
                step: *step,
                provenance: *provenance,
                error: Rc::clone(error),
                cancelled: *cancelled,
            },
            Self::WheelFormatFailed {
                attempt,
                delta,
                step,
                provenance,
                error,
                cancelled,
            } => Self::WheelFormatFailed {
                attempt: *attempt,
                delta: *delta,
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
        [
            NumericInputInteraction::WheelFailed {
                attempt: NumericWheelAttempt::Initial,
                provenance,
                cancelled: false,
                ..
            },
        ]
        | [
            NumericInputInteraction::WheelFormatFailed {
                attempt: NumericWheelAttempt::Initial,
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
                NumericInputInteraction::WheelFailed {
                    attempt: NumericWheelAttempt::Update,
                    provenance,
                    cancelled: true,
                    ..
                }
                | NumericInputInteraction::WheelFormatFailed {
                    attempt: NumericWheelAttempt::Update,
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
                | NumericInputInteraction::PointerFormatFailed { .. }
                | NumericInputInteraction::WheelFailed { .. }
                | NumericInputInteraction::WheelFormatFailed { .. } => false,
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
        [begin, update, commit]
            if begin.phase == EditPhase::Begin
                && update.phase == EditPhase::Update
                && commit.phase == EditPhase::Commit =>
        {
            is_pointer_provenance(begin.provenance)
                && is_pointer_provenance(update.provenance)
                && is_pointer_provenance(commit.provenance)
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
