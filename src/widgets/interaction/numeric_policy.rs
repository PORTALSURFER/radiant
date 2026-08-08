//! Private typed policy and codec boundaries for numeric controls.

#![allow(dead_code)]

use std::{fmt, ops::RangeInclusive};

/// The result of interpreting editable numeric text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NumericParseResult<T> {
    /// The draft is a valid prefix of an otherwise accepted value.
    Incomplete,
    /// The draft does not match the selected codec grammar.
    Invalid,
    /// The draft is valid text, but its value is outside the policy range.
    OutOfRange,
    /// The draft is valid text and its value is inside the policy range.
    Valid(T),
}

/// A private pair of parsing and canonical editable-formatting operations.
///
/// Implementations own their grammar and formatting representation. The
/// display-only [`super::ValueFormat`] policy is deliberately not part of this
/// contract and is never used to parse editable text.
pub(crate) trait NumericEditableCodec<T> {
    /// Interpret a draft without mutating any durable domain value.
    fn parse(&self, text: &str) -> NumericParseResult<T>;

    /// Write canonical editable text into caller-owned storage.
    fn format_editable(
        &self,
        value: T,
        output: &mut dyn fmt::Write,
    ) -> Result<(), NumericFormatError>;
}

/// Errors returned while constructing or using the private numeric policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NumericPolicyError {
    /// A policy range contains a nonfinite bound.
    NonFiniteBounds { min: f32, max: f32 },
    /// A policy range is not ordered.
    InvalidRange { min: f32, max: f32 },
}

/// Errors returned when canonical editable text cannot be produced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum NumericFormatError {
    /// The value supplied to the codec was not finite.
    NonFiniteValue,
    /// The value supplied to the codec is outside its inclusive range.
    OutOfRange { value: f32 },
    /// The caller-owned writer rejected a write.
    WriteFailed,
}

/// The first concrete private policy: invariant ASCII decimal `f32` text.
///
/// The policy accepts an optional leading sign, ASCII digits, one period
/// decimal separator, and an optional `e`/`E` exponent. It does not consult an
/// ambient locale and does not accept grouping, comma separators, or units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct InvariantF32Policy {
    min: f32,
    max: f32,
}

impl InvariantF32Policy {
    /// Create a policy over an inclusive finite range.
    pub(crate) fn new(range: RangeInclusive<f32>) -> Result<Self, NumericPolicyError> {
        let (min, max) = range.into_inner();
        if !min.is_finite() || !max.is_finite() {
            return Err(NumericPolicyError::NonFiniteBounds { min, max });
        }
        if min > max {
            return Err(NumericPolicyError::InvalidRange { min, max });
        }

        Ok(Self { min, max })
    }

    /// Return the inclusive lower bound.
    #[must_use]
    pub(crate) fn min(&self) -> f32 {
        self.min
    }

    /// Return the inclusive upper bound.
    #[must_use]
    pub(crate) fn max(&self) -> f32 {
        self.max
    }
}

impl NumericEditableCodec<f32> for InvariantF32Policy {
    fn parse(&self, text: &str) -> NumericParseResult<f32> {
        match classify_invariant_decimal(text) {
            LexicalResult::Incomplete => NumericParseResult::Incomplete,
            LexicalResult::Invalid => NumericParseResult::Invalid,
            LexicalResult::Complete => {
                let Ok(value) = text.parse::<f32>() else {
                    return NumericParseResult::Invalid;
                };
                if !value.is_finite() {
                    return NumericParseResult::Invalid;
                }
                if value < self.min || value > self.max {
                    NumericParseResult::OutOfRange
                } else {
                    NumericParseResult::Valid(value)
                }
            }
        }
    }

    fn format_editable(
        &self,
        value: f32,
        output: &mut dyn fmt::Write,
    ) -> Result<(), NumericFormatError> {
        if !value.is_finite() {
            return Err(NumericFormatError::NonFiniteValue);
        }
        if value < self.min || value > self.max {
            return Err(NumericFormatError::OutOfRange { value });
        }

        write!(output, "{value}").map_err(|_| NumericFormatError::WriteFailed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LexicalResult {
    Incomplete,
    Invalid,
    Complete,
}

fn classify_invariant_decimal(text: &str) -> LexicalResult {
    if text.is_empty() {
        return LexicalResult::Incomplete;
    }

    let bytes = text.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        index = 1;
        if index == bytes.len() {
            return LexicalResult::Incomplete;
        }
    }

    let integer_start = index;
    while matches!(bytes.get(index), Some(b'0'..=b'9')) {
        index += 1;
    }
    let has_integer_digits = index > integer_start;

    let mut has_decimal = false;
    let mut has_fraction_digits = false;
    if bytes.get(index) == Some(&b'.') {
        has_decimal = true;
        index += 1;
        let fraction_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        has_fraction_digits = index > fraction_start;
    }

    if !has_integer_digits && !has_fraction_digits {
        return if has_decimal && index == bytes.len() {
            LexicalResult::Incomplete
        } else {
            LexicalResult::Invalid
        };
    }
    if has_decimal && !has_fraction_digits {
        return if index == bytes.len() {
            LexicalResult::Incomplete
        } else {
            LexicalResult::Invalid
        };
    }

    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if index == exponent_start {
            return if index == bytes.len() {
                LexicalResult::Incomplete
            } else {
                LexicalResult::Invalid
            };
        }
    }

    if index == bytes.len() {
        LexicalResult::Complete
    } else {
        LexicalResult::Invalid
    }
}

impl fmt::Display for NumericPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteBounds { min, max } => {
                write!(
                    formatter,
                    "numeric policy bounds must be finite, got {min}..={max}"
                )
            }
            Self::InvalidRange { min, max } => {
                write!(
                    formatter,
                    "numeric policy range must be ordered, got {min}..={max}"
                )
            }
        }
    }
}

impl std::error::Error for NumericPolicyError {}

impl fmt::Display for NumericFormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => formatter.write_str("numeric format input must be finite"),
            Self::OutOfRange { value } => {
                write!(formatter, "numeric format input is out of range: {value}")
            }
            Self::WriteFailed => formatter.write_str("numeric format output write failed"),
        }
    }
}

impl std::error::Error for NumericFormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(min: f32, max: f32) -> InvariantF32Policy {
        InvariantF32Policy::new(min..=max).expect("test range is valid")
    }

    #[test]
    fn classifies_incomplete_invalid_out_of_range_and_valid_text() {
        let policy = policy(0.0, 1.0);

        assert_eq!(policy.parse(""), NumericParseResult::Incomplete);
        assert_eq!(policy.parse("1..0"), NumericParseResult::Invalid);
        assert_eq!(policy.parse("2"), NumericParseResult::OutOfRange);
        assert_eq!(policy.parse("0.5"), NumericParseResult::Valid(0.5));
    }

    #[test]
    fn preserves_incomplete_drafts_and_requires_invariant_grammar() {
        let policy = policy(-10.0, 10.0);

        for draft in ["+", "-", ".", "+.", "-.", "1.", "1e", "1e+", "1e-"] {
            assert_eq!(
                policy.parse(draft),
                NumericParseResult::Incomplete,
                "{draft}"
            );
        }
        for text in [" 1", "1 ", "1,5", "1_5", "1%", "NaN", "inf", "e1", "1.e2"] {
            assert_eq!(policy.parse(text), NumericParseResult::Invalid, "{text}");
        }
        assert_eq!(policy.parse("+.5"), NumericParseResult::Valid(0.5));
        assert_eq!(policy.parse("-1E+1"), NumericParseResult::Valid(-10.0));
    }

    #[test]
    fn rejects_nonfinite_values_and_invalid_policy_ranges() {
        let policy = policy(-f32::MAX, f32::MAX);

        assert_eq!(policy.parse("1e999"), NumericParseResult::Invalid);
        assert_eq!(
            InvariantF32Policy::new(f32::NEG_INFINITY..=1.0),
            Err(NumericPolicyError::NonFiniteBounds {
                min: f32::NEG_INFINITY,
                max: 1.0,
            })
        );
        assert_eq!(
            InvariantF32Policy::new(2.0..=1.0),
            Err(NumericPolicyError::InvalidRange { min: 2.0, max: 1.0 })
        );
        assert_eq!(policy.min(), -f32::MAX);
        assert_eq!(policy.max(), f32::MAX);
    }

    #[test]
    fn canonical_editable_format_round_trips_through_the_same_policy() {
        let policy = policy(-f32::MAX, f32::MAX);

        for value in [
            -20_000.5,
            -0.0,
            0.0,
            f32::MIN_POSITIVE,
            0.5,
            20_000.5,
            f32::MAX,
        ] {
            let mut output = String::new();
            policy
                .format_editable(value, &mut output)
                .expect("finite in-range value formats");
            assert_eq!(
                policy.parse(&output),
                NumericParseResult::Valid(value),
                "{value}: {output}"
            );
        }
    }

    #[test]
    fn formatting_rejects_nonfinite_out_of_range_and_failed_writes() {
        let policy = policy(0.0, 1.0);
        let mut output = String::new();

        assert_eq!(
            policy.format_editable(f32::NAN, &mut output),
            Err(NumericFormatError::NonFiniteValue)
        );
        assert_eq!(
            policy.format_editable(2.0, &mut output),
            Err(NumericFormatError::OutOfRange { value: 2.0 })
        );

        struct FailingWriter;
        impl fmt::Write for FailingWriter {
            fn write_str(&mut self, _: &str) -> fmt::Result {
                Err(fmt::Error)
            }
        }

        assert_eq!(
            policy.format_editable(0.5, &mut FailingWriter),
            Err(NumericFormatError::WriteFailed)
        );
    }
}
