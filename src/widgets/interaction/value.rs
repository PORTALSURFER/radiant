//! Deterministic mappings between normalized control positions and finite values.

use std::{fmt, ops::RangeInclusive};

/// A validated mapping between the normalized interval `0.0..=1.0` and a finite
/// domain range.
///
/// Construct mappings with [`ValueMapping::linear`] or
/// [`ValueMapping::logarithmic`]. Both constructors require finite, strictly
/// increasing bounds. Logarithmic mappings additionally require both bounds to
/// be positive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueMapping {
    kind: ValueMappingKind,
    min: f32,
    max: f32,
}

/// The curve used to map normalized positions to domain values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueMappingKind {
    /// A straight-line mapping between the domain bounds.
    Linear,
    /// A logarithmic mapping between positive domain bounds.
    Logarithmic,
}

/// Error returned when a value-mapping range cannot be validated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueMappingError {
    /// At least one range bound is not finite.
    NonFiniteBounds {
        /// Lower range bound supplied to the constructor.
        min: f32,
        /// Upper range bound supplied to the constructor.
        max: f32,
    },
    /// The range bounds are not strictly increasing.
    InvalidRange {
        /// Lower range bound supplied to the constructor.
        min: f32,
        /// Upper range bound supplied to the constructor.
        max: f32,
    },
    /// A logarithmic range contains a non-positive bound.
    NonPositiveLogarithmicBounds {
        /// Lower range bound supplied to the constructor.
        min: f32,
        /// Upper range bound supplied to the constructor.
        max: f32,
    },
}

impl ValueMapping {
    /// Create a finite linear mapping over an inclusive domain range.
    ///
    /// The bounds must be finite and strictly increasing. A normalized input
    /// of `0.0` maps to the lower bound and `1.0` maps to the upper bound.
    pub fn linear(range: RangeInclusive<f32>) -> Result<Self, ValueMappingError> {
        Self::new(range, ValueMappingKind::Linear)
    }

    /// Create a finite logarithmic mapping over an inclusive positive domain range.
    ///
    /// The bounds must be finite, strictly increasing, and positive. A
    /// normalized midpoint maps to the geometric midpoint of the bounds.
    pub fn logarithmic(range: RangeInclusive<f32>) -> Result<Self, ValueMappingError> {
        Self::new(range, ValueMappingKind::Logarithmic)
    }

    /// Return the mapping curve.
    #[must_use]
    pub fn kind(&self) -> ValueMappingKind {
        self.kind
    }

    /// Return the inclusive domain lower bound.
    #[must_use]
    pub fn min(&self) -> f32 {
        self.min
    }

    /// Return the inclusive domain upper bound.
    #[must_use]
    pub fn max(&self) -> f32 {
        self.max
    }

    /// Return the inclusive domain range.
    #[must_use]
    pub fn range(&self) -> RangeInclusive<f32> {
        self.min..=self.max
    }

    /// Map a normalized position to the domain value.
    ///
    /// Finite inputs outside `0.0..=1.0` are clamped to that interval.
    /// Nonfinite inputs return `None`.
    #[must_use]
    pub fn normalized_to_value(&self, normalized: f32) -> Option<f32> {
        if !normalized.is_finite() {
            return None;
        }

        let normalized = f64::from(normalized).clamp(0.0, 1.0);
        if normalized <= 0.0 {
            return Some(self.min);
        }
        if normalized >= 1.0 {
            return Some(self.max);
        }

        let min = f64::from(self.min);
        let max = f64::from(self.max);
        let value = match self.kind {
            ValueMappingKind::Linear => min + (max - min) * normalized,
            ValueMappingKind::Logarithmic => (min.ln() + (max.ln() - min.ln()) * normalized).exp(),
        };

        finite_f32(value)
    }

    /// Map a domain value to a normalized position.
    ///
    /// Finite inputs outside the mapping range are clamped to the nearest
    /// bound. Nonfinite inputs return `None`.
    #[must_use]
    pub fn value_to_normalized(&self, value: f32) -> Option<f32> {
        if !value.is_finite() {
            return None;
        }

        let min = f64::from(self.min);
        let max = f64::from(self.max);
        let value = f64::from(value).clamp(min, max);
        if value <= min {
            return Some(0.0);
        }
        if value >= max {
            return Some(1.0);
        }

        let normalized = match self.kind {
            ValueMappingKind::Linear => (value - min) / (max - min),
            ValueMappingKind::Logarithmic => (value.ln() - min.ln()) / (max.ln() - min.ln()),
        };

        finite_f32(normalized).map(|normalized| normalized.clamp(0.0, 1.0))
    }

    fn new(range: RangeInclusive<f32>, kind: ValueMappingKind) -> Result<Self, ValueMappingError> {
        let (min, max) = range.into_inner();
        if !min.is_finite() || !max.is_finite() {
            return Err(ValueMappingError::NonFiniteBounds { min, max });
        }
        if min >= max {
            return Err(ValueMappingError::InvalidRange { min, max });
        }
        if matches!(kind, ValueMappingKind::Logarithmic) && (min <= 0.0 || max <= 0.0) {
            return Err(ValueMappingError::NonPositiveLogarithmicBounds { min, max });
        }

        Ok(Self { kind, min, max })
    }
}

impl fmt::Display for ValueMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteBounds { min, max } => write!(
                formatter,
                "value mapping bounds must be finite, got {min}..={max}"
            ),
            Self::InvalidRange { min, max } => write!(
                formatter,
                "value mapping bounds must be strictly increasing, got {min}..={max}"
            ),
            Self::NonPositiveLogarithmicBounds { min, max } => write!(
                formatter,
                "logarithmic value mapping bounds must be positive, got {min}..={max}"
            ),
        }
    }
}

impl std::error::Error for ValueMappingError {}

fn finite_f32(value: f64) -> Option<f32> {
    let value = value as f32;
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_mapping_has_finite_endpoints_and_f64_intermediate_behavior() {
        let mapping = ValueMapping::linear(-10_000_000.0..=10_000_000.0).unwrap();

        assert_eq!(mapping.normalized_to_value(0.0), Some(-10_000_000.0));
        assert_eq!(mapping.normalized_to_value(1.0), Some(10_000_000.0));
        assert_eq!(mapping.normalized_to_value(0.5), Some(0.0));
        assert_eq!(mapping.value_to_normalized(-10_000_000.0), Some(0.0));
        assert_eq!(mapping.value_to_normalized(10_000_000.0), Some(1.0));
        assert_eq!(mapping.value_to_normalized(0.0), Some(0.5));
    }

    #[test]
    fn logarithmic_mapping_uses_geometric_midpoint() {
        let mapping = ValueMapping::logarithmic(20.0..=20_000.0).unwrap();
        let midpoint = mapping.normalized_to_value(0.5).unwrap();

        assert!((midpoint - 632.4555).abs() < 0.001);
        assert!((mapping.value_to_normalized(midpoint).unwrap() - 0.5).abs() < 0.000_001);
    }

    #[test]
    fn accessors_expose_mapping_definition() {
        let mapping = ValueMapping::logarithmic(20.0..=20_000.0).unwrap();

        assert_eq!(mapping.kind(), ValueMappingKind::Logarithmic);
        assert_eq!(mapping.min(), 20.0);
        assert_eq!(mapping.max(), 20_000.0);
        assert_eq!(mapping.range(), 20.0..=20_000.0);
    }

    #[test]
    fn value_mapping_kind_is_available_through_the_public_widgets_surface() {
        let mapping = crate::widgets::ValueMapping::linear(0.0..=1.0).unwrap();
        let kind: crate::widgets::ValueMappingKind = mapping.kind();

        assert_eq!(kind, crate::widgets::ValueMappingKind::Linear);
    }

    #[test]
    fn finite_inputs_are_clamped_to_the_mapping_boundaries() {
        let mapping = ValueMapping::linear(10.0..=20.0).unwrap();

        assert_eq!(mapping.normalized_to_value(-2.0), Some(10.0));
        assert_eq!(mapping.normalized_to_value(2.0), Some(20.0));
        assert_eq!(mapping.value_to_normalized(0.0), Some(0.0));
        assert_eq!(mapping.value_to_normalized(30.0), Some(1.0));
    }

    #[test]
    fn nonfinite_inputs_are_rejected() {
        let mapping = ValueMapping::linear(10.0..=20.0).unwrap();

        for input in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
            assert_eq!(mapping.normalized_to_value(input), None);
            assert_eq!(mapping.value_to_normalized(input), None);
        }
    }

    #[test]
    fn constructors_reject_invalid_ranges_with_typed_errors() {
        assert!(matches!(
            ValueMapping::linear(f32::NEG_INFINITY..=1.0),
            Err(ValueMappingError::NonFiniteBounds { .. })
        ));
        assert!(matches!(
            ValueMapping::linear(1.0..=1.0),
            Err(ValueMappingError::InvalidRange { .. })
        ));
        assert!(matches!(
            ValueMapping::linear(2.0..=1.0),
            Err(ValueMappingError::InvalidRange { .. })
        ));
        assert!(matches!(
            ValueMapping::logarithmic(0.0..=20.0),
            Err(ValueMappingError::NonPositiveLogarithmicBounds { .. })
        ));
        assert!(matches!(
            ValueMapping::logarithmic(-20.0..=-2.0),
            Err(ValueMappingError::NonPositiveLogarithmicBounds { .. })
        ));
    }
}
