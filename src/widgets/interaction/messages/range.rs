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

/// One ordered event in a keyboard knob automation gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnobAutomationEvent {
    /// Keyboard gesture began at the pre-edit value.
    GestureStarted {
        /// Value before the key edit.
        value: f32,
    },
    /// Keyboard gesture's final clamped value.
    ValueChanged {
        /// Final normalized value after clamping.
        value: f32,
    },
    /// Keyboard gesture ended at the final value.
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
}

impl KnobKeyboardGesture {
    /// Build a complete keyboard lifecycle batch.
    pub const fn new(start_value: f32, final_value: f32) -> Self {
        Self {
            events: [
                KnobAutomationEvent::GestureStarted { value: start_value },
                KnobAutomationEvent::ValueChanged { value: final_value },
                KnobAutomationEvent::GestureEnded { value: final_value },
            ],
        }
    }
}

/// Explicit host-automation lifecycle emitted by a radial knob.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KnobMessage {
    /// Pointer or keyboard gesture began at the current normalized value.
    GestureStarted {
        /// Value at gesture start.
        value: f32,
    },
    /// The normalized value changed during an active gesture.
    ValueChanged {
        /// Latest normalized value.
        value: f32,
    },
    /// Pointer or keyboard gesture ended at the current normalized value.
    GestureEnded {
        /// Value at gesture end.
        value: f32,
    },
    /// The control returned to its configured default value.
    Reset {
        /// Default normalized value restored by the reset gesture.
        value: f32,
    },
    /// Ordered keyboard lifecycle batch for host automation.
    KeyboardGesture(KnobKeyboardGesture),
}
