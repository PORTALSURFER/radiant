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

    /// Project the latest effective update or meaningful rollback value.
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
        latest_update.or(rollback)
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
