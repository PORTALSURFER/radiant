//! Generic adjustment policies for numeric controls.
//!
//! An adjustment owns the domain mapping and all non-text changes made by a
//! numeric control. It is deliberately separate from [`super::NumericCodec`]
//! and from the control's edit lifecycle.

#![allow(dead_code)]

use super::value::{ValueMapping, ValueMappingError};
use std::{fmt, ops::RangeInclusive};

/// The semantic step size selected by an adjustment action.
///
/// Applications may map these names to their own domain-specific step values,
/// but each policy must declare all three values explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericStep {
    /// The ordinary arrow-key, wheel, or scrub step.
    Base,
    /// The fine-grained step, conventionally selected by Shift.
    Fine,
    /// The coarse-grained step, conventionally selected by Command or
    /// Control depending on the host platform.
    Coarse,
}

/// The direction of a discrete numeric adjustment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericStepDirection {
    /// Move toward the lower declared domain boundary.
    Decrease,
    /// Move toward the upper declared domain boundary.
    Increase,
}

/// A caller-provided policy for mapping and adjusting a numeric domain.
///
/// Implementations own a finite domain, a total monotonic mapping from the
/// normalized interval, a checked inverse, explicit Base/Fine/Coarse steps,
/// and bounded pure sensitivities for scrubbing and wheel input. Finite
/// adjustment inputs clamp only at the policy's declared boundaries. The
/// methods do not parse text, consult application state, emit edit events, or
/// own a transaction lifecycle.
pub trait NumericAdjustment<T> {
    /// The error returned when a policy cannot produce a safe result.
    type Error;

    /// Map a normalized position to a domain value.
    ///
    /// Finite values outside `0.0..=1.0` are clamped by the policy. Nonfinite
    /// input and a nonfinite mapping result must return `Self::Error`.
    fn normalized_to_value(&self, normalized: f32) -> Result<T, Self::Error>;

    /// Map a domain value to a normalized position using the checked inverse.
    ///
    /// Finite values outside the declared domain may be clamped to its nearest
    /// boundary. Nonfinite input and a nonfinite inverse result must return
    /// `Self::Error`.
    fn value_to_normalized(&self, value: &T) -> Result<f32, Self::Error>;

    /// Apply one explicit Base/Fine/Coarse step in a direction.
    fn step(
        &self,
        value: &T,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<T, Self::Error>;

    /// Apply a pure pointer-scrub displacement using the policy's bounded
    /// sensitivity for `step`.
    fn scrub(&self, value: &T, normalized_delta: f32, step: NumericStep) -> Result<T, Self::Error>;

    /// Apply a pure signed wheel delta using the policy's bounded sensitivity
    /// and selected step.
    fn wheel(&self, value: &T, delta: f32, step: NumericStep) -> Result<T, Self::Error>;
}

const MAX_SENSITIVITY: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepSpace {
    Domain,
    Normalized,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NumericAdjustmentPolicyError {
    NonFiniteBounds { min: f32, max: f32 },
    InvalidRange { min: f32, max: f32 },
    NonPositiveLogarithmicBounds { min: f32, max: f32 },
    NonFiniteStep { step: NumericStep, value: f32 },
    NonPositiveStep { step: NumericStep, value: f32 },
    NonFiniteSensitivity { step: NumericStep, value: f32 },
    UnboundedSensitivity { step: NumericStep, value: f32 },
    NonFiniteInput { value: f32 },
    MappingFailure,
}

/// Private executable evidence for the generic adjustment boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InvariantF32Adjustment {
    mapping: ValueMapping,
    steps: [f32; 3],
    sensitivities: [f32; 3],
    step_space: StepSpace,
}

impl InvariantF32Adjustment {
    pub(crate) fn linear(
        range: RangeInclusive<f32>,
        steps: [f32; 3],
        sensitivities: [f32; 3],
    ) -> Result<Self, NumericAdjustmentPolicyError> {
        Self::new(
            ValueMapping::linear(range).map_err(NumericAdjustmentPolicyError::from),
            steps,
            sensitivities,
            StepSpace::Domain,
        )
    }

    pub(crate) fn logarithmic(
        range: RangeInclusive<f32>,
        steps: [f32; 3],
        sensitivities: [f32; 3],
    ) -> Result<Self, NumericAdjustmentPolicyError> {
        Self::new(
            ValueMapping::logarithmic(range).map_err(NumericAdjustmentPolicyError::from),
            steps,
            sensitivities,
            StepSpace::Normalized,
        )
    }

    fn new(
        mapping: Result<ValueMapping, NumericAdjustmentPolicyError>,
        steps: [f32; 3],
        sensitivities: [f32; 3],
        step_space: StepSpace,
    ) -> Result<Self, NumericAdjustmentPolicyError> {
        let mapping = mapping?;
        for (step, value) in NumericStep::ALL.into_iter().zip(steps) {
            if !value.is_finite() {
                return Err(NumericAdjustmentPolicyError::NonFiniteStep { step, value });
            }
            if value <= 0.0 {
                return Err(NumericAdjustmentPolicyError::NonPositiveStep { step, value });
            }
        }
        for (step, value) in NumericStep::ALL.into_iter().zip(sensitivities) {
            if !value.is_finite() {
                return Err(NumericAdjustmentPolicyError::NonFiniteSensitivity { step, value });
            }
            if value > MAX_SENSITIVITY {
                return Err(NumericAdjustmentPolicyError::UnboundedSensitivity { step, value });
            }
            if value <= 0.0 {
                return Err(NumericAdjustmentPolicyError::NonFiniteSensitivity { step, value });
            }
        }

        Ok(Self {
            mapping,
            steps,
            sensitivities,
            step_space,
        })
    }

    fn step_index(step: NumericStep) -> usize {
        match step {
            NumericStep::Base => 0,
            NumericStep::Fine => 1,
            NumericStep::Coarse => 2,
        }
    }

    fn step_size(&self, step: NumericStep) -> f32 {
        self.steps[Self::step_index(step)]
    }

    fn sensitivity(&self, step: NumericStep) -> f32 {
        self.sensitivities[Self::step_index(step)]
    }

    fn checked_input(value: f32) -> Result<f32, NumericAdjustmentPolicyError> {
        value
            .is_finite()
            .then_some(value)
            .ok_or(NumericAdjustmentPolicyError::NonFiniteInput { value })
    }

    fn map_normalized(&self, normalized: f32) -> Result<f32, NumericAdjustmentPolicyError> {
        Self::checked_input(normalized)?;
        self.map_normalized_f64(f64::from(normalized))
    }

    fn map_normalized_f64(&self, normalized: f64) -> Result<f32, NumericAdjustmentPolicyError> {
        if !normalized.is_finite() {
            return Err(NumericAdjustmentPolicyError::MappingFailure);
        }
        let normalized = normalized.clamp(0.0, 1.0) as f32;
        self.mapping
            .normalized_to_value(normalized)
            .ok_or(NumericAdjustmentPolicyError::MappingFailure)
    }

    fn map_value(&self, value: f32) -> Result<f32, NumericAdjustmentPolicyError> {
        Self::checked_input(value)?;
        self.mapping
            .value_to_normalized(value)
            .ok_or(NumericAdjustmentPolicyError::MappingFailure)
    }

    fn map_delta(
        &self,
        value: f32,
        delta: f32,
        step: NumericStep,
        sensitivity: f32,
    ) -> Result<f32, NumericAdjustmentPolicyError> {
        Self::checked_input(value)?;
        Self::checked_input(delta)?;
        let normalized = self.map_value(value)?;
        let scaled = f64::from(delta) * f64::from(self.step_size(step)) * f64::from(sensitivity);
        let normalized = f64::from(normalized) + scaled;
        if !normalized.is_finite() {
            return Err(NumericAdjustmentPolicyError::MappingFailure);
        }
        self.map_normalized_f64(normalized)
    }
}

impl NumericAdjustment<f32> for InvariantF32Adjustment {
    type Error = NumericAdjustmentPolicyError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        Self::checked_input(normalized)?;
        self.mapping
            .normalized_to_value(normalized)
            .ok_or(NumericAdjustmentPolicyError::MappingFailure)
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        self.map_value(*value)
    }

    fn step(
        &self,
        value: &f32,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Self::checked_input(*value)?;
        let sign = match direction {
            NumericStepDirection::Decrease => -1.0,
            NumericStepDirection::Increase => 1.0,
        };
        match self.step_space {
            StepSpace::Domain => {
                let value = f64::from(*value)
                    .clamp(f64::from(self.mapping.min()), f64::from(self.mapping.max()));
                let next = value + sign * f64::from(self.step_size(step));
                Ok(next.clamp(f64::from(self.mapping.min()), f64::from(self.mapping.max())) as f32)
            }
            StepSpace::Normalized => {
                let normalized = f64::from(self.map_value(*value)?);
                let next = normalized + sign * f64::from(self.step_size(step));
                self.map_normalized_f64(next)
            }
        }
    }

    fn scrub(
        &self,
        value: &f32,
        normalized_delta: f32,
        step: NumericStep,
    ) -> Result<f32, Self::Error> {
        self.map_delta(*value, normalized_delta, step, self.sensitivity(step))
    }

    fn wheel(&self, value: &f32, delta: f32, step: NumericStep) -> Result<f32, Self::Error> {
        Self::checked_input(*value)?;
        Self::checked_input(delta)?;
        match self.step_space {
            StepSpace::Domain => {
                let current = f64::from(*value)
                    .clamp(f64::from(self.mapping.min()), f64::from(self.mapping.max()));
                let change = f64::from(delta)
                    * f64::from(self.step_size(step))
                    * f64::from(self.sensitivity(step));
                Ok((current + change)
                    .clamp(f64::from(self.mapping.min()), f64::from(self.mapping.max()))
                    as f32)
            }
            StepSpace::Normalized => {
                let normalized = f64::from(self.map_value(*value)?);
                let next = normalized
                    + f64::from(delta)
                        * f64::from(self.step_size(step))
                        * f64::from(self.sensitivity(step));
                self.map_normalized_f64(next)
            }
        }
    }
}

impl NumericStep {
    const ALL: [Self; 3] = [Self::Base, Self::Fine, Self::Coarse];
}

impl From<ValueMappingError> for NumericAdjustmentPolicyError {
    fn from(error: ValueMappingError) -> Self {
        match error {
            ValueMappingError::NonFiniteBounds { min, max } => Self::NonFiniteBounds { min, max },
            ValueMappingError::InvalidRange { min, max } => Self::InvalidRange { min, max },
            ValueMappingError::NonPositiveLogarithmicBounds { min, max } => {
                Self::NonPositiveLogarithmicBounds { min, max }
            }
        }
    }
}

impl fmt::Display for NumericAdjustmentPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteBounds { min, max } => {
                write!(
                    formatter,
                    "numeric adjustment bounds must be finite: {min}..={max}"
                )
            }
            Self::InvalidRange { min, max } => {
                write!(
                    formatter,
                    "numeric adjustment range must increase: {min}..={max}"
                )
            }
            Self::NonPositiveLogarithmicBounds { min, max } => write!(
                formatter,
                "logarithmic numeric adjustment bounds must be positive: {min}..={max}"
            ),
            Self::NonFiniteStep { step, value } => {
                write!(
                    formatter,
                    "numeric adjustment {step:?} step is not finite: {value}"
                )
            }
            Self::NonPositiveStep { step, value } => {
                write!(
                    formatter,
                    "numeric adjustment {step:?} step is not positive: {value}"
                )
            }
            Self::NonFiniteSensitivity { step, value } => write!(
                formatter,
                "numeric adjustment {step:?} sensitivity is not finite and positive: {value}"
            ),
            Self::UnboundedSensitivity { step, value } => write!(
                formatter,
                "numeric adjustment {step:?} sensitivity exceeds the bound: {value}"
            ),
            Self::NonFiniteInput { value } => {
                write!(formatter, "numeric adjustment input is not finite: {value}")
            }
            Self::MappingFailure => formatter.write_str("numeric adjustment mapping failed"),
        }
    }
}

impl std::error::Error for NumericAdjustmentPolicyError {}

#[cfg(test)]
mod tests {
    use super::super::value::ValueMappingKind;
    use super::*;

    fn linear() -> InvariantF32Adjustment {
        InvariantF32Adjustment::linear(0.0..=100.0, [1.0, 0.1, 10.0], [1.0, 0.1, 10.0])
            .expect("test policy is valid")
    }

    fn logarithmic() -> InvariantF32Adjustment {
        InvariantF32Adjustment::logarithmic(20.0..=20_000.0, [0.01, 0.001, 0.1], [1.0, 0.1, 10.0])
            .expect("test policy is valid")
    }

    #[test]
    fn rejects_invalid_ranges_steps_and_sensitivities() {
        assert!(matches!(
            InvariantF32Adjustment::linear(f32::NAN..=1.0, [1.0; 3], [1.0; 3]),
            Err(NumericAdjustmentPolicyError::NonFiniteBounds { .. })
        ));
        assert!(matches!(
            InvariantF32Adjustment::logarithmic(-1.0..=1.0, [1.0; 3], [1.0; 3]),
            Err(NumericAdjustmentPolicyError::NonPositiveLogarithmicBounds { .. })
        ));
        assert!(matches!(
            InvariantF32Adjustment::linear(0.0..=1.0, [0.0, 1.0, 1.0], [1.0; 3]),
            Err(NumericAdjustmentPolicyError::NonPositiveStep { .. })
        ));
        assert!(matches!(
            InvariantF32Adjustment::linear(0.0..=1.0, [1.0; 3], [17.0, 1.0, 1.0]),
            Err(NumericAdjustmentPolicyError::UnboundedSensitivity { .. })
        ));
    }

    #[test]
    fn linear_mapping_is_finite_and_round_trips() {
        let adjustment = linear();

        assert_eq!(adjustment.normalized_to_value(0.0), Ok(0.0));
        assert_eq!(adjustment.normalized_to_value(1.0), Ok(100.0));
        assert_eq!(adjustment.normalized_to_value(-2.0), Ok(0.0));
        assert_eq!(adjustment.normalized_to_value(2.0), Ok(100.0));
        assert_eq!(adjustment.value_to_normalized(&50.0), Ok(0.5));
    }

    #[test]
    fn logarithmic_mapping_uses_normalized_steps() {
        let adjustment = logarithmic();
        let midpoint = adjustment.normalized_to_value(0.5).unwrap();

        assert!((midpoint - 632.4555).abs() < 0.001);
        assert!((adjustment.value_to_normalized(&midpoint).unwrap() - 0.5).abs() < 0.000_001);
        assert_eq!(
            adjustment.step(&20_000.0, NumericStepDirection::Increase, NumericStep::Base),
            Ok(20_000.0)
        );
    }

    #[test]
    fn rejects_nonfinite_inputs() {
        let adjustment = linear();

        assert!(matches!(
            adjustment.normalized_to_value(f32::NAN),
            Err(NumericAdjustmentPolicyError::NonFiniteInput { .. })
        ));
        assert!(matches!(
            adjustment.value_to_normalized(&f32::INFINITY),
            Err(NumericAdjustmentPolicyError::NonFiniteInput { .. })
        ));
        assert!(matches!(
            adjustment.scrub(&50.0, f32::INFINITY, NumericStep::Base),
            Err(NumericAdjustmentPolicyError::NonFiniteInput { .. })
        ));
    }

    #[test]
    fn discrete_steps_clamp_at_declared_boundaries() {
        let adjustment = linear();

        assert_eq!(
            adjustment.step(&0.0, NumericStepDirection::Decrease, NumericStep::Base),
            Ok(0.0)
        );
        assert_eq!(
            adjustment.step(&99.0, NumericStepDirection::Increase, NumericStep::Base),
            Ok(100.0)
        );
        assert_eq!(
            adjustment.step(&50.0, NumericStepDirection::Decrease, NumericStep::Fine),
            Ok(49.9)
        );
    }

    #[test]
    fn scrub_uses_bounded_step_sensitivity_and_clamps() {
        let adjustment = linear();

        let scrubbed = adjustment
            .scrub(&50.0, 0.1, NumericStep::Base)
            .expect("finite scrub should map");
        assert!((scrubbed - 60.0).abs() < 0.000_01);
        assert_eq!(adjustment.scrub(&99.0, 0.1, NumericStep::Coarse), Ok(100.0));
    }

    #[test]
    fn wheel_applies_selected_step_and_clamps() {
        let adjustment = linear();

        assert_eq!(adjustment.wheel(&50.0, 2.0, NumericStep::Base), Ok(52.0));
        assert_eq!(adjustment.wheel(&1.0, -20.0, NumericStep::Base), Ok(0.0));
    }

    #[test]
    fn public_trait_is_implemented_by_private_policy() {
        let adjustment = linear();
        let _: &dyn NumericAdjustment<f32, Error = NumericAdjustmentPolicyError> = &adjustment;
    }

    #[test]
    fn value_mapping_kind_remains_available_for_policy_evidence() {
        let adjustment = logarithmic();
        assert_eq!(adjustment.mapping.kind(), ValueMappingKind::Logarithmic);
    }
}
