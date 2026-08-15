use super::{KnobKeyboardMetadata, KnobPointerMetadata, KnobWheelMetadata};
use crate::widgets::interaction::InteractionProvenance;

/// A typed failure while translating a knob value between normalized and
/// application-owned domain space.
#[derive(Debug, PartialEq)]
pub enum KnobDomainError<E> {
    /// The adjustment rejected the initial or default domain-to-normalized
    /// inverse.
    ValueToNormalized {
        /// Adjustment-provided inverse-mapping failure.
        error: E,
    },
    /// The adjustment rejected a normalized-to-domain candidate.
    NormalizedToValue {
        /// Adjustment-provided forward-mapping failure.
        error: E,
    },
    /// A domain value supplied to or returned from the adjustment was not
    /// finite.
    NonFiniteValue {
        /// The nonfinite domain value.
        value: f32,
    },
    /// An adjustment returned a nonfinite normalized value.
    NonFiniteNormalized {
        /// The nonfinite normalized value.
        normalized: f32,
    },
    /// An adjustment returned a normalized value outside `0.0..=1.0`.
    NormalizedOutOfRange {
        /// The out-of-range normalized value.
        normalized: f32,
    },
}

impl<E: Clone> Clone for KnobDomainError<E> {
    fn clone(&self) -> Self {
        match self {
            Self::ValueToNormalized { error } => Self::ValueToNormalized {
                error: error.clone(),
            },
            Self::NormalizedToValue { error } => Self::NormalizedToValue {
                error: error.clone(),
            },
            Self::NonFiniteValue { value } => Self::NonFiniteValue { value: *value },
            Self::NonFiniteNormalized { normalized } => Self::NonFiniteNormalized {
                normalized: *normalized,
            },
            Self::NormalizedOutOfRange { normalized } => Self::NormalizedOutOfRange {
                normalized: *normalized,
            },
        }
    }
}

/// The domain-space interaction that attempted a forward mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnobDomainMappingAttempt {
    /// A candidate from an active pointer gesture.
    PointerUpdate,
    /// An atomic keyboard gesture candidate.
    KeyboardGesture,
    /// An atomic wheel gesture candidate.
    WheelGesture,
}

/// The explicit reason a domain-space pointer gesture was cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnobDomainCancellationReason {
    /// Keyboard focus left the active knob gesture.
    FocusLoss,
    /// The runtime revoked pointer capture from the active gesture.
    PointerCaptureLoss,
    /// The knob became disabled or read-only while the gesture was active.
    DisabledOrReadOnly,
}

/// One ordered domain-space event in a keyboard or wheel gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnobDomainAutomationEvent {
    /// The gesture began at the pre-edit domain value.
    GestureStarted {
        /// Domain value at gesture start.
        value: f32,
    },
    /// The gesture accepted a domain value.
    ValueChanged {
        /// Accepted domain value.
        value: f32,
    },
    /// The gesture ended at its final domain value.
    GestureEnded {
        /// Final domain value.
        value: f32,
    },
}

/// Compound keyboard automation lifecycle in domain space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobDomainKeyboardGesture {
    /// Exactly three ordered domain events: start, value, and end.
    pub events: [KnobDomainAutomationEvent; 3],
    /// Metadata from the accepted keyboard input sample.
    pub metadata: KnobKeyboardMetadata,
}

impl KnobDomainKeyboardGesture {
    /// Build a complete keyboard lifecycle batch without native metadata.
    pub const fn new(start_value: f32, final_value: f32) -> Self {
        Self::new_with_metadata(start_value, final_value, KnobKeyboardMetadata::empty())
    }

    /// Build a complete keyboard lifecycle batch with native metadata.
    pub const fn new_with_metadata(
        start_value: f32,
        final_value: f32,
        metadata: KnobKeyboardMetadata,
    ) -> Self {
        Self {
            events: [
                KnobDomainAutomationEvent::GestureStarted { value: start_value },
                KnobDomainAutomationEvent::ValueChanged { value: final_value },
                KnobDomainAutomationEvent::GestureEnded { value: final_value },
            ],
            metadata,
        }
    }

    /// Return metadata captured at the keyboard input boundary.
    pub const fn input_metadata(&self) -> KnobKeyboardMetadata {
        self.metadata
    }
}

/// Compound wheel automation lifecycle in domain space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobDomainWheelGesture {
    /// Exactly three ordered domain events: start, value, and end.
    pub events: [KnobDomainAutomationEvent; 3],
    /// Metadata from the accepted wheel input sample.
    pub metadata: KnobWheelMetadata,
}

impl KnobDomainWheelGesture {
    /// Build a complete wheel lifecycle batch without native metadata.
    pub const fn new(start_value: f32, final_value: f32) -> Self {
        Self::new_with_metadata(start_value, final_value, KnobWheelMetadata::empty())
    }

    /// Build a complete wheel lifecycle batch with native metadata.
    pub const fn new_with_metadata(
        start_value: f32,
        final_value: f32,
        metadata: KnobWheelMetadata,
    ) -> Self {
        Self {
            events: [
                KnobDomainAutomationEvent::GestureStarted { value: start_value },
                KnobDomainAutomationEvent::ValueChanged { value: final_value },
                KnobDomainAutomationEvent::GestureEnded { value: final_value },
            ],
            metadata,
        }
    }

    /// Return metadata captured at the wheel input boundary.
    pub const fn input_metadata(&self) -> KnobWheelMetadata {
        self.metadata
    }
}

/// Typed domain-space lifecycle emitted by a mapped radial knob.
#[derive(Debug, PartialEq)]
pub enum KnobDomainMessage<E> {
    /// A pointer gesture began at the current domain value.
    GestureStarted {
        /// Domain value at gesture start.
        value: f32,
        /// Pointer metadata from the accepted press.
        metadata: KnobPointerMetadata,
    },
    /// A pointer gesture accepted a new domain value.
    ValueChanged {
        /// Latest accepted domain value.
        value: f32,
        /// Pointer metadata from the accepted move.
        metadata: KnobPointerMetadata,
    },
    /// A pointer gesture ended at the retained domain value.
    GestureEnded {
        /// Domain value at gesture end.
        value: f32,
        /// Pointer metadata from the terminal input.
        metadata: KnobPointerMetadata,
    },
    /// An active pointer gesture was cancelled and restored its start value.
    GestureCancelled {
        /// Domain value captured when the gesture began.
        start_value: f32,
        /// Domain value retained immediately before restoration.
        previous_value: f32,
        /// Explicit cancellation boundary.
        reason: KnobDomainCancellationReason,
        /// Pointer metadata from the cancellation input.
        metadata: KnobPointerMetadata,
    },
    /// A reset restored the cached domain default.
    Reset {
        /// Domain value before the reset.
        previous_value: f32,
        /// Cached domain default restored by the reset.
        value: f32,
        /// Pointer metadata from the accepted double-click.
        metadata: KnobPointerMetadata,
    },
    /// Atomic keyboard lifecycle in domain space.
    KeyboardGesture(KnobDomainKeyboardGesture),
    /// Atomic wheel lifecycle in domain space.
    WheelGesture(KnobDomainWheelGesture),
    /// A forward mapping attempt failed without publishing a partial gesture.
    MappingFailed {
        /// The input category that produced the candidate.
        attempt: KnobDomainMappingAttempt,
        /// Normalized candidate presented to the mapping boundary.
        normalized: f32,
        /// Domain value retained after the failed attempt.
        retained_value: f32,
        /// Full provenance of the input that produced the candidate.
        provenance: InteractionProvenance,
        /// Typed mapping failure.
        error: KnobDomainError<E>,
    },
}

impl<E: Clone> Clone for KnobDomainMessage<E> {
    fn clone(&self) -> Self {
        match self {
            Self::GestureStarted { value, metadata } => Self::GestureStarted {
                value: *value,
                metadata: *metadata,
            },
            Self::ValueChanged { value, metadata } => Self::ValueChanged {
                value: *value,
                metadata: *metadata,
            },
            Self::GestureEnded { value, metadata } => Self::GestureEnded {
                value: *value,
                metadata: *metadata,
            },
            Self::GestureCancelled {
                start_value,
                previous_value,
                reason,
                metadata,
            } => Self::GestureCancelled {
                start_value: *start_value,
                previous_value: *previous_value,
                reason: *reason,
                metadata: *metadata,
            },
            Self::Reset {
                previous_value,
                value,
                metadata,
            } => Self::Reset {
                previous_value: *previous_value,
                value: *value,
                metadata: *metadata,
            },
            Self::KeyboardGesture(gesture) => Self::KeyboardGesture(*gesture),
            Self::WheelGesture(gesture) => Self::WheelGesture(*gesture),
            Self::MappingFailed {
                attempt,
                normalized,
                retained_value,
                provenance,
                error,
            } => Self::MappingFailed {
                attempt: *attempt,
                normalized: *normalized,
                retained_value: *retained_value,
                provenance: *provenance,
                error: error.clone(),
            },
        }
    }
}
