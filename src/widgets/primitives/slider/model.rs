//! Slider data model types.

/// Immutable slider configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    /// Normalized amount applied for each arrow-key step.
    pub keyboard_step: f32,
}

/// Mutable slider interaction state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderState {
    /// Current normalized value in the inclusive range `0.0..=1.0`.
    pub value: f32,
}
