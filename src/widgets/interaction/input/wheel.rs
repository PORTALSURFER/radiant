//! Exact, backend-neutral wheel samples.

use super::{PointerModifiers, WidgetInput};
use crate::gui::{
    input::{InputSequenceRange, InputTimestamp},
    types::Vector2,
};

/// Logical pixels represented by one validated wheel line in scroll-offset
/// direction.
pub const WHEEL_LINE_EQUIVALENCE_PIXELS: f32 = 40.0;

/// Error returned when a wheel delta cannot preserve finite unit evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelDeltaError {
    /// One or more components are non-finite, or line projection would overflow.
    NonFinite,
}

/// Unit-qualified wheel displacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    /// A signed displacement expressed in logical lines in scroll-offset
    /// direction.
    ///
    /// Positive components increase the corresponding logical scroll offset.
    Lines(Vector2),
    /// A signed displacement expressed in logical pixels in scroll-offset
    /// direction.
    ///
    /// Positive components increase the corresponding logical scroll offset.
    Pixels(Vector2),
}

impl WheelDelta {
    /// Construct a validated line delta.
    pub fn lines(delta: Vector2) -> Result<Self, WheelDeltaError> {
        Self::try_lines(delta)
    }

    /// Construct a validated pixel delta.
    pub fn pixels(delta: Vector2) -> Result<Self, WheelDeltaError> {
        Self::try_pixels(delta)
    }

    /// Construct a validated line delta.
    pub fn try_lines(delta: Vector2) -> Result<Self, WheelDeltaError> {
        let delta = validate_delta(delta)?;
        let projected = Vector2::new(
            delta.x * WHEEL_LINE_EQUIVALENCE_PIXELS,
            delta.y * WHEEL_LINE_EQUIVALENCE_PIXELS,
        );
        is_finite(projected)
            .then_some(Self::Lines(delta))
            .ok_or(WheelDeltaError::NonFinite)
    }

    /// Construct a validated pixel delta.
    pub fn try_pixels(delta: Vector2) -> Result<Self, WheelDeltaError> {
        validate_delta(delta).map(Self::Pixels)
    }

    /// Return the original unit-qualified displacement.
    pub const fn vector(self) -> Vector2 {
        match self {
            Self::Lines(delta) | Self::Pixels(delta) => delta,
        }
    }

    /// Return whether this value carries finite, representable unit evidence.
    pub fn is_valid(self) -> bool {
        match self {
            Self::Lines(delta) => {
                is_finite(delta)
                    && is_finite(Vector2::new(
                        delta.x * WHEEL_LINE_EQUIVALENCE_PIXELS,
                        delta.y * WHEEL_LINE_EQUIVALENCE_PIXELS,
                    ))
            }
            Self::Pixels(delta) => is_finite(delta),
        }
    }

    /// Project this delta into the legacy logical-pixel wheel contract without
    /// changing its scroll-offset direction.
    pub fn to_logical_pixels(self) -> Option<Vector2> {
        self.is_valid().then(|| match self {
            Self::Lines(delta) => Vector2::new(
                delta.x * WHEEL_LINE_EQUIVALENCE_PIXELS,
                delta.y * WHEEL_LINE_EQUIVALENCE_PIXELS,
            ),
            Self::Pixels(delta) => delta,
        })
    }

    /// Build the legacy pixel variant without changing the old raw-vector API.
    pub(crate) const fn legacy_pixels(delta: Vector2) -> Self {
        Self::Pixels(delta)
    }
}

fn validate_delta(delta: Vector2) -> Result<Vector2, WheelDeltaError> {
    is_finite(delta)
        .then_some(delta)
        .ok_or(WheelDeltaError::NonFinite)
}

fn is_finite(delta: Vector2) -> bool {
    delta.x.is_finite() && delta.y.is_finite()
}

/// Explicit lifecycle evidence carried by an exact wheel sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelPhase {
    /// A new managed wheel sequence may be admitted.
    Started,
    /// A continuation of an admitted wheel sequence.
    Changed,
    /// A normal terminal boundary for an admitted wheel sequence.
    Ended,
    /// A cancelling terminal boundary for an admitted wheel sequence.
    Cancelled,
    /// One bounded wheel gesture without retained sequence authority.
    Discrete,
}

/// Error returned when exact wheel-sample evidence is malformed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WheelSampleError {
    /// The unit-qualified delta was not valid.
    Delta(WheelDeltaError),
}

/// Backend-neutral wheel input with exact unit and phase evidence.
///
/// Its delta is already expressed in logical scroll-offset direction: native
/// adapters negate platform content-direction deltas exactly once. The exact
/// evidence remains available to qualified widget or policy routing; an
/// ordinary scroll-container fallback may project it to one logical-pixel axis
/// and does not retain the phase.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelSample {
    delta: WheelDelta,
    phase: Option<WheelPhase>,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

impl WheelSample {
    /// Construct a validated sample with optional explicit phase evidence.
    pub fn new(
        delta: WheelDelta,
        phase: Option<WheelPhase>,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new_with_metadata(delta, phase, modifiers, None, None)
    }

    /// Construct a validated sample with optional explicit phase evidence.
    pub fn try_new(
        delta: WheelDelta,
        phase: Option<WheelPhase>,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, phase, modifiers)
    }

    /// Construct a validated sample while preserving optional native metadata.
    pub fn new_with_metadata(
        delta: WheelDelta,
        phase: Option<WheelPhase>,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Result<Self, WheelSampleError> {
        delta
            .is_valid()
            .then_some(Self::from_parts(
                delta,
                phase,
                modifiers,
                timestamp,
                sequence_range,
            ))
            .ok_or(WheelSampleError::Delta(WheelDeltaError::NonFinite))
    }

    /// Construct a validated phase-less sample.
    pub fn phase_less(
        delta: WheelDelta,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, None, modifiers)
    }

    /// Construct a validated discrete sample.
    pub fn discrete(
        delta: WheelDelta,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, Some(WheelPhase::Discrete), modifiers)
    }

    /// Construct a validated started sample.
    pub fn started(
        delta: WheelDelta,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, Some(WheelPhase::Started), modifiers)
    }

    /// Construct a validated changed sample.
    pub fn changed(
        delta: WheelDelta,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, Some(WheelPhase::Changed), modifiers)
    }

    /// Construct a validated ended sample.
    pub fn ended(delta: WheelDelta, modifiers: PointerModifiers) -> Result<Self, WheelSampleError> {
        Self::new(delta, Some(WheelPhase::Ended), modifiers)
    }

    /// Construct a validated cancelled sample.
    pub fn cancelled(
        delta: WheelDelta,
        modifiers: PointerModifiers,
    ) -> Result<Self, WheelSampleError> {
        Self::new(delta, Some(WheelPhase::Cancelled), modifiers)
    }

    /// Return the exact unit-qualified delta.
    pub const fn delta(self) -> WheelDelta {
        self.delta
    }

    /// Return explicit phase evidence, or `None` for a phase-less sample.
    pub const fn phase(self) -> Option<WheelPhase> {
        self.phase
    }

    /// Return the modifiers captured with this sample.
    pub const fn modifiers(self) -> PointerModifiers {
        self.modifiers
    }

    /// Return the optional native timestamp carried as provenance.
    pub const fn timestamp(self) -> Option<InputTimestamp> {
        self.timestamp
    }

    /// Return the optional native sequence range carried as provenance.
    pub const fn sequence_range(self) -> Option<InputSequenceRange> {
        self.sequence_range
    }

    /// Return whether this sample still carries valid exact evidence.
    pub fn is_valid(self) -> bool {
        self.delta.is_valid()
    }

    pub(crate) fn from_parts(
        delta: WheelDelta,
        phase: Option<WheelPhase>,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Self {
        Self {
            delta,
            phase,
            modifiers,
            timestamp,
            sequence_range,
        }
    }

    pub(crate) fn to_widget_input(self, position: crate::gui::types::Point) -> Option<WidgetInput> {
        self.delta.to_logical_pixels().map(|delta| {
            WidgetInput::wheel_with_metadata(
                position,
                delta,
                self.modifiers,
                self.timestamp,
                self.sequence_range,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Point;

    #[test]
    fn validated_delta_constructors_preserve_units_and_reject_nonfinite_values() {
        let lines = WheelDelta::lines(Vector2::new(1.0, -2.0)).expect("finite lines");
        let pixels = WheelDelta::pixels(Vector2::new(3.5, -4.5)).expect("finite pixels");

        assert_eq!(lines, WheelDelta::Lines(Vector2::new(1.0, -2.0)));
        assert_eq!(pixels, WheelDelta::Pixels(Vector2::new(3.5, -4.5)));
        assert_eq!(lines.to_logical_pixels(), Some(Vector2::new(40.0, -80.0)));
        assert_eq!(pixels.to_logical_pixels(), Some(Vector2::new(3.5, -4.5)));
        assert_eq!(
            WheelDelta::pixels(Vector2::new(f32::NAN, 0.0)),
            Err(WheelDeltaError::NonFinite)
        );
        assert_eq!(
            WheelDelta::lines(Vector2::new(f32::MAX, 0.0)),
            Err(WheelDeltaError::NonFinite)
        );
    }

    #[test]
    fn sample_preserves_phase_and_projects_only_for_legacy_dispatch() {
        let delta = WheelDelta::pixels(Vector2::new(0.25, -1.5)).expect("finite pixels");
        let sample = WheelSample::new(
            delta,
            Some(WheelPhase::Started),
            PointerModifiers {
                command: true,
                shift: false,
                alt: true,
            },
        )
        .expect("valid sample");

        assert_eq!(sample.delta(), delta);
        assert_eq!(sample.phase(), Some(WheelPhase::Started));
        assert_eq!(sample.timestamp(), None);
        assert_eq!(sample.sequence_range(), None);
        assert_eq!(
            sample
                .to_widget_input(Point::new(8.0, 9.0))
                .expect("legacy projection"),
            WidgetInput::Wheel {
                position: Point::new(8.0, 9.0),
                delta: Vector2::new(0.25, -1.5),
                modifiers: sample.modifiers(),
                timestamp: None,
                sequence_range: None,
            }
        );
    }
}
