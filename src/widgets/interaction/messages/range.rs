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
}
