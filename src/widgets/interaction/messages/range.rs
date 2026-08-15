use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    widgets::interaction::{EditEvent, EditPhase, EditTransaction, PointerModifiers},
};

/// Message emitted by a reusable scrollbar primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollbarMessage {
    /// The viewport offset changed to the provided normalized fraction.
    OffsetChanged {
        /// Clamped normalized viewport start in the inclusive range `0.0..=1.0`.
        offset_fraction: f32,
    },
}

/// Message emitted by a reusable slider primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderMessage {
    /// The normalized slider value changed.
    ValueChanged {
        /// Clamped normalized value in the inclusive range `0.0..=1.0`.
        value: f32,
    },
}

/// A typed failure while translating a slider value between normalized and
/// domain space.
///
/// The error is also used by [`crate::application::slider_domain`] when the
/// initial domain value cannot be admitted. Mapping failures never clamp or
/// replace the supplied evidence.
#[derive(Debug, PartialEq)]
pub enum SliderDomainError<E> {
    /// The adjustment rejected the initial domain-to-normalized inverse.
    ValueToNormalized {
        /// Adjustment-provided inverse-mapping failure.
        error: E,
    },
    /// The adjustment rejected a normalized-to-domain candidate.
    NormalizedToValue {
        /// Adjustment-provided forward-mapping failure.
        error: E,
    },
    /// A domain value supplied to or returned from the adjustment was not finite.
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

impl<E: Clone> Clone for SliderDomainError<E> {
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

/// Message emitted by a retained slider with an application-owned `f32`
/// domain adjustment.
#[derive(Debug, PartialEq)]
pub enum SliderDomainMessage<E> {
    /// A normalized slider candidate was mapped to the domain successfully.
    ValueChanged {
        /// The accepted mapped domain value.
        value: f32,
    },
    /// A normalized candidate could not be mapped to the domain.
    MappingFailed {
        /// The normalized candidate supplied to the mapping boundary.
        normalized: f32,
        /// The typed mapping failure.
        error: SliderDomainError<E>,
    },
}

impl<E: Clone> Clone for SliderDomainMessage<E> {
    fn clone(&self) -> Self {
        match self {
            Self::ValueChanged { value } => Self::ValueChanged { value: *value },
            Self::MappingFailed { normalized, error } => Self::MappingFailed {
                normalized: *normalized,
                error: error.clone(),
            },
        }
    }
}

/// One bounded, ordered batch of shared edit events emitted by a slider.
///
/// A batch contains between one and three events and never allocates.  The
/// trailing storage is private implementation capacity; [`Self::events`] only
/// exposes the ordered events that belong to the batch.  Slider input creates
/// at most one batch for each accepted input sample, so the batch is also the
/// runtime's one-output boundary for one interaction.
#[derive(Clone, Copy)]
pub struct SliderEditBatch {
    events: [EditEvent<f32>; 3],
    len: u8,
    meaningful_rollback: bool,
}

impl std::fmt::Debug for SliderEditBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SliderEditBatch")
            .field("events", &self.events())
            .field("meaningful_rollback", &self.meaningful_rollback)
            .finish()
    }
}

impl PartialEq for SliderEditBatch {
    fn eq(&self, other: &Self) -> bool {
        self.events() == other.events() && self.meaningful_rollback == other.meaningful_rollback
    }
}

impl SliderEditBatch {
    /// Maximum number of ordered events carried by one slider batch.
    pub const MAX_EVENTS: usize = 3;

    /// Build a one-event batch.
    pub fn new(event: EditEvent<f32>) -> Self {
        Self {
            events: [event; Self::MAX_EVENTS],
            len: 1,
            meaningful_rollback: false,
        }
    }

    /// Build a one-event batch.
    pub fn single(event: EditEvent<f32>) -> Self {
        Self::new(event)
    }

    /// Build a batch from one to three ordered events.
    ///
    /// The events must share one transaction.  An empty or over-capacity slice
    /// and a mixed-transaction slice return `None`.
    pub fn from_events(events: &[EditEvent<f32>]) -> Option<Self> {
        if !(1..=Self::MAX_EVENTS).contains(&events.len()) {
            return None;
        }
        let transaction = events.first()?.transaction;
        if events.iter().any(|event| event.transaction != transaction) {
            return None;
        }

        let mut stored = [events[0]; Self::MAX_EVENTS];
        for (slot, event) in stored.iter_mut().zip(events.iter().copied()) {
            *slot = event;
        }
        let meaningful_rollback = events.iter().enumerate().any(|(index, event)| {
            event.phase == EditPhase::Cancel && has_effective_update(&events[..index])
        });
        Some(Self {
            events: stored,
            len: events.len() as u8,
            meaningful_rollback,
        })
    }

    /// Return the ordered edit events in this batch.
    pub fn events(&self) -> &[EditEvent<f32>] {
        &self.events[..usize::from(self.len)]
    }

    /// Return the number of ordered events in this batch.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether this batch contains no events.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the transaction shared by every event in this batch.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }

    /// Project a meaningful rollback or the latest effective update.
    ///
    /// Lifecycle-only boundaries (`Begin` and `Commit`) do not project a
    /// concise value.  A slider that cancels without changing its value does
    /// not emit a batch; the internal rollback constructor marks the emitted
    /// cancellation as meaningful without adding another event to the batch.
    pub fn value_change(&self) -> Option<f32> {
        let mut latest_update = None;
        let mut rollback = None;
        let mut current_value = None;
        for event in self.events() {
            match event.phase {
                EditPhase::Begin => current_value = Some(event.value),
                EditPhase::Update => {
                    let previous_value = current_value.unwrap_or(event.start_value);
                    if values_differ(previous_value, event.value) {
                        latest_update = Some(event.value);
                    }
                    current_value = Some(event.value);
                }
                EditPhase::Commit => current_value = Some(event.value),
                EditPhase::Cancel if self.meaningful_rollback => rollback = Some(event.start_value),
                EditPhase::Cancel => {}
            }
        }
        rollback.or(latest_update)
    }

    pub(crate) fn rollback(event: EditEvent<f32>) -> Self {
        Self {
            events: [event; Self::MAX_EVENTS],
            len: 1,
            meaningful_rollback: true,
        }
    }
}

fn has_effective_update(events: &[EditEvent<f32>]) -> bool {
    let mut current_value = None;
    for event in events {
        match event.phase {
            EditPhase::Begin => current_value = Some(event.value),
            EditPhase::Update => {
                let previous_value = current_value.unwrap_or(event.start_value);
                if values_differ(previous_value, event.value) {
                    return true;
                }
                current_value = Some(event.value);
            }
            EditPhase::Commit | EditPhase::Cancel => {}
        }
    }
    false
}

fn values_differ(left: f32, right: f32) -> bool {
    if left.is_finite() && right.is_finite() {
        (left - right).abs() > f32::EPSILON
    } else {
        left != right
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnobEditKind {
    Pointer,
    Keyboard,
    Wheel,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnobCancellationReason {
    FocusLoss,
    PointerCapture,
}

/// One bounded, ordered batch of shared edit events emitted by a knob.
///
/// The storage has room for every lifecycle phase while each accepted knob
/// input emits at most three events: pointer boundaries carry one or two
/// events, and keyboard, wheel, and reset inputs carry an atomic
/// `Begin`/`Update`/`Commit` batch. The private kind preserves the legacy
/// [`KnobMessage`] projection without adding state to the public widget.
#[derive(Clone, Copy)]
pub struct KnobEditBatch {
    events: [EditEvent<f32>; 4],
    len: u8,
    kind: KnobEditKind,
    meaningful_rollback: bool,
    cancellation_reason: Option<KnobCancellationReason>,
    legacy_terminal_value: Option<f32>,
}

impl std::fmt::Debug for KnobEditBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KnobEditBatch")
            .field("events", &self.events())
            .field("meaningful_rollback", &self.meaningful_rollback)
            .finish()
    }
}

impl PartialEq for KnobEditBatch {
    fn eq(&self, other: &Self) -> bool {
        self.events() == other.events() && self.meaningful_rollback == other.meaningful_rollback
    }
}

impl KnobEditBatch {
    /// Maximum number of ordered events carried by one knob batch.
    pub const MAX_EVENTS: usize = 4;

    /// Build a one-event batch with the kind inferred from its provenance.
    pub fn new(event: EditEvent<f32>) -> Self {
        Self {
            events: [event; Self::MAX_EVENTS],
            len: 1,
            kind: inferred_kind(event.provenance),
            meaningful_rollback: false,
            cancellation_reason: None,
            legacy_terminal_value: None,
        }
    }

    /// Build a one-event batch with the kind inferred from its provenance.
    pub fn single(event: EditEvent<f32>) -> Self {
        Self::new(event)
    }

    /// Build a batch from one to four ordered events.
    ///
    /// The events must share one transaction. An empty or over-capacity slice
    /// and a mixed-transaction slice return `None`.
    pub fn from_events(events: &[EditEvent<f32>]) -> Option<Self> {
        Self::from_events_with_kind(events, inferred_kind(events.first()?.provenance))
    }

    /// Return the ordered edit events in this batch.
    pub fn events(&self) -> &[EditEvent<f32>] {
        &self.events[..usize::from(self.len)]
    }

    /// Return the number of ordered events in this batch.
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether this batch contains no events.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the transaction shared by every event in this batch.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }

    /// Project the latest accepted update or a meaningful cancellation
    /// rollback.
    ///
    /// Lifecycle-only pointer boundaries do not project a concise value. A
    /// reset remains observable even when it already equals its configured
    /// default, matching the legacy `KnobMessage::Reset` contract.
    pub fn value_change(&self) -> Option<f32> {
        let mut latest_update = None;
        let mut rollback = None;
        for event in self.events() {
            match event.phase {
                EditPhase::Begin | EditPhase::Commit => {}
                EditPhase::Update => latest_update = Some(event.value),
                EditPhase::Cancel if self.meaningful_rollback => rollback = Some(event.start_value),
                EditPhase::Cancel => {}
            }
        }
        rollback.or(latest_update)
    }

    pub(crate) fn pointer(events: &[EditEvent<f32>]) -> Option<Self> {
        Self::from_events_with_kind(events, KnobEditKind::Pointer)
    }

    pub(crate) fn keyboard(events: &[EditEvent<f32>]) -> Option<Self> {
        Self::from_events_with_kind(events, KnobEditKind::Keyboard)
    }

    pub(crate) fn wheel(events: &[EditEvent<f32>]) -> Option<Self> {
        Self::from_events_with_kind(events, KnobEditKind::Wheel)
    }

    pub(crate) fn reset(events: &[EditEvent<f32>]) -> Option<Self> {
        Self::from_events_with_kind(events, KnobEditKind::Reset)
    }

    /// Build a batch whose terminal cancel event represents a meaningful
    /// rollback to its starting value.
    ///
    /// The explicit constructor keeps the rollback meaning reproducible when
    /// the batch contains only its terminal `Cancel` event.
    pub fn rollback(event: EditEvent<f32>) -> Self {
        Self {
            events: [event; Self::MAX_EVENTS],
            len: 1,
            kind: KnobEditKind::Pointer,
            meaningful_rollback: true,
            cancellation_reason: None,
            legacy_terminal_value: None,
        }
    }

    pub(crate) fn focus_loss(
        event: EditEvent<f32>,
        meaningful_rollback: bool,
        legacy_terminal_value: f32,
    ) -> Self {
        Self::cancellation(
            event,
            meaningful_rollback,
            KnobCancellationReason::FocusLoss,
            legacy_terminal_value,
        )
    }

    pub(crate) fn pointer_capture(
        event: EditEvent<f32>,
        meaningful_rollback: bool,
        legacy_terminal_value: f32,
    ) -> Self {
        Self::cancellation(
            event,
            meaningful_rollback,
            KnobCancellationReason::PointerCapture,
            legacy_terminal_value,
        )
    }

    pub(crate) fn legacy_message(&self) -> Option<KnobMessage> {
        match self.kind {
            KnobEditKind::Pointer => self.pointer_message(),
            KnobEditKind::Keyboard => self.keyboard_message(),
            KnobEditKind::Wheel => self.wheel_message(),
            KnobEditKind::Reset => self.reset_message(),
        }
    }

    fn from_events_with_kind(events: &[EditEvent<f32>], kind: KnobEditKind) -> Option<Self> {
        if !(1..=Self::MAX_EVENTS).contains(&events.len()) {
            return None;
        }
        let transaction = events.first()?.transaction;
        if events.iter().any(|event| event.transaction != transaction) {
            return None;
        }

        let mut stored = [events[0]; Self::MAX_EVENTS];
        for (slot, event) in stored.iter_mut().zip(events.iter().copied()) {
            *slot = event;
        }
        let meaningful_rollback = events.iter().enumerate().any(|(index, event)| {
            event.phase == EditPhase::Cancel && has_effective_update(&events[..index])
        });
        Some(Self {
            events: stored,
            len: events.len() as u8,
            kind,
            meaningful_rollback,
            cancellation_reason: None,
            legacy_terminal_value: None,
        })
    }

    fn cancellation(
        event: EditEvent<f32>,
        meaningful_rollback: bool,
        cancellation_reason: KnobCancellationReason,
        legacy_terminal_value: f32,
    ) -> Self {
        Self {
            events: [event; Self::MAX_EVENTS],
            len: 1,
            kind: KnobEditKind::Pointer,
            meaningful_rollback,
            cancellation_reason: Some(cancellation_reason),
            legacy_terminal_value: Some(legacy_terminal_value),
        }
    }

    fn pointer_message(&self) -> Option<KnobMessage> {
        let event = self.events().last()?;
        if event.phase == EditPhase::Cancel
            && self.cancellation_reason == Some(KnobCancellationReason::PointerCapture)
        {
            return None;
        }
        let metadata = pointer_metadata(event.provenance);
        Some(match event.phase {
            EditPhase::Begin => KnobMessage::GestureStarted {
                value: event.value,
                metadata,
            },
            EditPhase::Update => KnobMessage::ValueChanged {
                value: event.value,
                metadata,
            },
            EditPhase::Commit => KnobMessage::GestureEnded {
                value: event.value,
                metadata,
            },
            EditPhase::Cancel => KnobMessage::GestureEnded {
                value: self.legacy_terminal_value.unwrap_or(event.value),
                metadata,
            },
        })
    }

    fn keyboard_message(&self) -> Option<KnobMessage> {
        let start_value = self.events().first()?.start_value;
        let final_value = self.events().last()?.value;
        let timestamp = self
            .events()
            .iter()
            .find_map(|event| match event.provenance {
                crate::widgets::interaction::InteractionProvenance::Keyboard { timestamp } => {
                    Some(timestamp)
                }
                _ => None,
            })
            .flatten();
        Some(KnobMessage::KeyboardGesture(
            KnobKeyboardGesture::new_with_metadata(
                start_value,
                final_value,
                KnobKeyboardMetadata { timestamp },
            ),
        ))
    }

    fn wheel_message(&self) -> Option<KnobMessage> {
        let start_value = self.events().first()?.start_value;
        let final_value = self.events().last()?.value;
        let metadata = self
            .events()
            .last()
            .map(|event| wheel_metadata(event.provenance))?;
        Some(KnobMessage::WheelGesture(
            KnobWheelGesture::new_with_metadata(start_value, final_value, metadata),
        ))
    }

    fn reset_message(&self) -> Option<KnobMessage> {
        let event = self.events().last()?;
        Some(KnobMessage::Reset {
            value: event.value,
            metadata: pointer_metadata(event.provenance),
        })
    }
}

fn inferred_kind(provenance: crate::widgets::interaction::InteractionProvenance) -> KnobEditKind {
    match provenance {
        crate::widgets::interaction::InteractionProvenance::Keyboard { .. } => {
            KnobEditKind::Keyboard
        }
        _ => KnobEditKind::Pointer,
    }
}

fn pointer_metadata(
    provenance: crate::widgets::interaction::InteractionProvenance,
) -> KnobPointerMetadata {
    match provenance {
        crate::widgets::interaction::InteractionProvenance::Pointer {
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

fn wheel_metadata(
    provenance: crate::widgets::interaction::InteractionProvenance,
) -> KnobWheelMetadata {
    match provenance {
        crate::widgets::interaction::InteractionProvenance::Pointer {
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

/// One ordered event in a knob automation gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnobAutomationEvent {
    /// Gesture began at the pre-edit value.
    GestureStarted {
        /// Value before the key edit.
        value: f32,
    },
    /// Gesture's final clamped value.
    ValueChanged {
        /// Final normalized value after clamping.
        value: f32,
    },
    /// Gesture ended at the final value.
    GestureEnded {
        /// Final normalized value after clamping.
        value: f32,
    },
}

/// Compound keyboard automation lifecycle preserving event ordering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobKeyboardGesture {
    /// Exactly three ordered events: start, final value, end.
    pub events: [KnobAutomationEvent; 3],
    /// Normalized provenance from the accepted keyboard input sample.
    pub metadata: KnobKeyboardMetadata,
}

impl KnobKeyboardGesture {
    /// Build a complete keyboard lifecycle batch.
    pub const fn new(start_value: f32, final_value: f32) -> Self {
        Self::new_with_metadata(start_value, final_value, KnobKeyboardMetadata::empty())
    }

    /// Build a complete keyboard lifecycle batch with normalized input provenance.
    pub const fn new_with_metadata(
        start_value: f32,
        final_value: f32,
        metadata: KnobKeyboardMetadata,
    ) -> Self {
        Self {
            events: [
                KnobAutomationEvent::GestureStarted { value: start_value },
                KnobAutomationEvent::ValueChanged { value: final_value },
                KnobAutomationEvent::GestureEnded { value: final_value },
            ],
            metadata,
        }
    }

    /// Return normalized input provenance carried by this keyboard gesture.
    pub const fn input_metadata(&self) -> KnobKeyboardMetadata {
        self.metadata
    }
}

/// Normalized input provenance carried by a keyboard automation gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnobKeyboardMetadata {
    /// Optional timestamp captured at the native input boundary.
    pub timestamp: Option<InputTimestamp>,
}

impl KnobKeyboardMetadata {
    /// Build metadata with no native sample provenance.
    pub const fn empty() -> Self {
        Self { timestamp: None }
    }
}

/// Normalized input provenance carried by a wheel automation gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnobWheelMetadata {
    /// Modifier state captured with the normalized wheel input sample.
    pub modifiers: PointerModifiers,
    /// Optional timestamp captured at the native input boundary.
    pub timestamp: Option<InputTimestamp>,
    /// Optional opaque native sample sequence range.
    pub sequence_range: Option<InputSequenceRange>,
}

impl KnobWheelMetadata {
    /// Build metadata with no native sample provenance.
    pub const fn empty() -> Self {
        Self {
            modifiers: PointerModifiers {
                command: false,
                shift: false,
                alt: false,
            },
            timestamp: None,
            sequence_range: None,
        }
    }
}

/// Compound wheel automation lifecycle preserving event ordering.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnobWheelGesture {
    /// Exactly three ordered events: start, value, end.
    pub events: [KnobAutomationEvent; 3],
    /// Normalized provenance from the accepted wheel input sample.
    pub metadata: KnobWheelMetadata,
}

impl KnobWheelGesture {
    /// Build a complete wheel lifecycle batch.
    pub const fn new(start_value: f32, final_value: f32) -> Self {
        Self::new_with_metadata(start_value, final_value, KnobWheelMetadata::empty())
    }

    /// Build a complete wheel lifecycle batch with normalized input provenance.
    pub const fn new_with_metadata(
        start_value: f32,
        final_value: f32,
        metadata: KnobWheelMetadata,
    ) -> Self {
        Self {
            events: [
                KnobAutomationEvent::GestureStarted { value: start_value },
                KnobAutomationEvent::ValueChanged { value: final_value },
                KnobAutomationEvent::GestureEnded { value: final_value },
            ],
            metadata,
        }
    }

    /// Return normalized input provenance carried by this wheel gesture.
    pub const fn input_metadata(&self) -> KnobWheelMetadata {
        self.metadata
    }
}

/// Normalized input provenance carried by an incremental pointer gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnobPointerMetadata {
    /// Modifier state captured with the current normalized pointer sample.
    pub modifiers: PointerModifiers,
    /// Optional timestamp captured at the native input boundary.
    pub timestamp: Option<InputTimestamp>,
    /// Optional opaque native sample sequence range.
    pub sequence_range: Option<InputSequenceRange>,
}

impl KnobPointerMetadata {
    /// Build metadata with no native sample provenance.
    pub const fn empty() -> Self {
        Self {
            modifiers: PointerModifiers {
                command: false,
                shift: false,
                alt: false,
            },
            timestamp: None,
            sequence_range: None,
        }
    }
}

/// Explicit host-automation lifecycle emitted by a radial knob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnobMessage {
    /// Pointer gesture began at the current normalized value.
    GestureStarted {
        /// Value at gesture start.
        value: f32,
        /// Normalized provenance from the accepted pointer press.
        metadata: KnobPointerMetadata,
    },
    /// The normalized value changed during an active pointer gesture.
    ValueChanged {
        /// Latest normalized value.
        value: f32,
        /// Normalized provenance from the accepted captured pointer move.
        metadata: KnobPointerMetadata,
    },
    /// Pointer gesture ended at the current normalized value.
    GestureEnded {
        /// Value at gesture end.
        value: f32,
        /// Normalized provenance from the terminal pointer input.
        metadata: KnobPointerMetadata,
    },
    /// The control returned to its configured default value.
    Reset {
        /// Default normalized value restored by the reset gesture.
        value: f32,
        /// Normalized provenance from the accepted primary double-click.
        metadata: KnobPointerMetadata,
    },
    /// Ordered keyboard lifecycle batch for host automation.
    KeyboardGesture(KnobKeyboardGesture),
    /// Ordered wheel lifecycle batch for host automation.
    WheelGesture(KnobWheelGesture),
}

impl KnobMessage {
    /// Return normalized provenance carried by a pointer gesture.
    pub const fn pointer_gesture_metadata(&self) -> Option<KnobPointerMetadata> {
        match self {
            Self::GestureStarted { metadata, .. }
            | Self::ValueChanged { metadata, .. }
            | Self::GestureEnded { metadata, .. }
            | Self::Reset { metadata, .. } => Some(*metadata),
            Self::KeyboardGesture(_) | Self::WheelGesture(_) => None,
        }
    }
}
