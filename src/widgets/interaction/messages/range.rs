use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    widgets::interaction::PointerModifiers,
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
