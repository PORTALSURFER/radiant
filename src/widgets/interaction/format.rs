//! Backend-neutral policies for displaying common numeric values.

use std::fmt::{self, Write as _};

const MAX_FRACTION_DIGITS_LIMIT: u8 = 9;
const DEFAULT_FREQUENCY_FRACTION_DIGITS: u8 = 2;

/// The common numeric form selected by a [`ValueFormat`] policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueFormatKind {
    /// Display the value as a decimal number.
    Decimal,
    /// Multiply the value by 100 and append a percent sign.
    Percent,
    /// Display the value and append a ` Hz` suffix.
    Frequency,
}

/// The decimal separator used by a [`ValueFormat`] policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecimalSeparator {
    /// Use a period (`.`) as the decimal separator.
    Period,
    /// Use a comma (`,`) as the decimal separator.
    Comma,
}

/// An error returned while writing a formatted value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueFormatError {
    /// The value supplied to [`ValueFormat::write_into`] was not finite.
    NonFiniteValue,
    /// The caller-owned formatter rejected a write.
    WriteFailed,
}

/// A bounded, backend-neutral policy for displaying common numeric values.
///
/// The policy stores only its form, fixed fractional precision, and explicit
/// decimal separator. Fractional precision is capped at nine digits; a larger
/// requested `u8` value is clamped to that cap. Formatting never discovers or
/// consults an ambient operating-system locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueFormat {
    kind: ValueFormatKind,
    fraction_digits: u8,
    separator: DecimalSeparator,
}

impl ValueFormat {
    /// The largest number of fractional digits accepted by this policy.
    ///
    /// Constructors clamp larger `u8` requests to this value.
    pub const MAX_FRACTION_DIGITS: u8 = MAX_FRACTION_DIGITS_LIMIT;

    /// Create a decimal policy with the requested number of fractional digits.
    ///
    /// Requests above the bounded nine-digit maximum are clamped to nine.
    #[must_use]
    pub fn decimal(fraction_digits: u8) -> Self {
        Self::new(ValueFormatKind::Decimal, fraction_digits)
    }

    /// Create a percent policy with the requested number of fractional digits.
    ///
    /// The input is multiplied by 100 before formatting, and a percent sign is
    /// appended. Requests above the bounded nine-digit maximum are clamped to
    /// nine.
    #[must_use]
    pub fn percent(fraction_digits: u8) -> Self {
        Self::new(ValueFormatKind::Percent, fraction_digits)
    }

    /// Create a frequency policy with the deterministic default precision of
    /// two fractional digits.
    #[must_use]
    pub fn frequency() -> Self {
        Self::frequency_with_digits(DEFAULT_FREQUENCY_FRACTION_DIGITS)
    }

    /// Create a frequency policy with the requested number of fractional
    /// digits.
    ///
    /// The value is formatted with a ` Hz` suffix. Requests above the bounded
    /// nine-digit maximum are clamped to nine.
    #[must_use]
    pub fn frequency_with_digits(fraction_digits: u8) -> Self {
        Self::new(ValueFormatKind::Frequency, fraction_digits)
    }

    /// Return a copy of this policy using the supplied decimal separator.
    #[must_use]
    pub fn with_decimal_separator(mut self, separator: DecimalSeparator) -> Self {
        self.separator = separator;
        self
    }

    /// Return the selected numeric form.
    #[must_use]
    pub fn kind(&self) -> ValueFormatKind {
        self.kind
    }

    /// Return the bounded number of fractional digits.
    #[must_use]
    pub fn fraction_digits(&self) -> u8 {
        self.fraction_digits
    }

    /// Return the explicit decimal separator.
    #[must_use]
    pub fn decimal_separator(&self) -> DecimalSeparator {
        self.separator
    }

    /// Write a formatted value into caller-owned `fmt::Write` storage.
    ///
    /// Decimal values are written as-is, percent values are multiplied by 100
    /// and suffixed with `%`, and frequency values are suffixed with ` Hz`.
    /// Fixed precision is bounded by nine fractional digits. Nonfinite input
    /// is rejected before any write is attempted, and a writer error is mapped
    /// to [`ValueFormatError::WriteFailed`].
    pub fn write_into(
        &self,
        value: f32,
        output: &mut impl fmt::Write,
    ) -> Result<(), ValueFormatError> {
        if !value.is_finite() {
            return Err(ValueFormatError::NonFiniteValue);
        }

        let value = f64::from(value);
        let value = match self.kind {
            ValueFormatKind::Decimal | ValueFormatKind::Frequency => value,
            ValueFormatKind::Percent => value * 100.0,
        };
        if !value.is_finite() {
            return Err(ValueFormatError::NonFiniteValue);
        }

        let precision = usize::from(self.fraction_digits);
        let numeric_result = {
            let mut numeric_output = DecimalSeparatorWriter {
                output,
                separator: self.separator,
            };
            write!(&mut numeric_output, "{value:.precision$}")
        };
        numeric_result.map_err(|_| ValueFormatError::WriteFailed)?;

        let suffix = match self.kind {
            ValueFormatKind::Decimal => "",
            ValueFormatKind::Percent => "%",
            ValueFormatKind::Frequency => " Hz",
        };
        output
            .write_str(suffix)
            .map_err(|_| ValueFormatError::WriteFailed)
    }

    fn new(kind: ValueFormatKind, fraction_digits: u8) -> Self {
        Self {
            kind,
            fraction_digits: fraction_digits.min(Self::MAX_FRACTION_DIGITS),
            separator: DecimalSeparator::Period,
        }
    }
}

struct DecimalSeparatorWriter<'a, W: fmt::Write + ?Sized> {
    output: &'a mut W,
    separator: DecimalSeparator,
}

impl<W: fmt::Write + ?Sized> fmt::Write for DecimalSeparatorWriter<'_, W> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.separator == DecimalSeparator::Period {
            return self.output.write_str(value);
        }

        let mut segment_start = 0;
        for (index, character) in value.char_indices() {
            if character == '.' {
                self.output.write_str(&value[segment_start..index])?;
                self.output.write_char(',')?;
                segment_start = index + character.len_utf8();
            }
        }

        self.output.write_str(&value[segment_start..])
    }
}

impl fmt::Display for ValueFormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteValue => formatter.write_str("value format input must be finite"),
            Self::WriteFailed => formatter.write_str("value format output write failed"),
        }
    }
}

impl std::error::Error for ValueFormatError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(policy: ValueFormat, value: f32) -> Result<String, ValueFormatError> {
        let mut output = String::new();
        policy.write_into(value, &mut output)?;
        Ok(output)
    }

    #[test]
    fn writes_decimal_percent_and_frequency_forms() {
        assert_eq!(
            render(ValueFormat::decimal(2), 1.25),
            Ok(String::from("1.25"))
        );
        assert_eq!(
            render(ValueFormat::percent(2), 0.125),
            Ok(String::from("12.50%"))
        );
        assert_eq!(
            render(ValueFormat::frequency(), 440.0),
            Ok(String::from("440.00 Hz"))
        );
    }

    #[test]
    fn applies_requested_precision_to_each_form() {
        assert_eq!(
            render(ValueFormat::decimal(0), 12.6),
            Ok(String::from("13"))
        );
        assert_eq!(
            render(ValueFormat::decimal(3), 1.23456),
            Ok(String::from("1.235"))
        );
        assert_eq!(
            render(ValueFormat::percent(1), 0.12345),
            Ok(String::from("12.3%"))
        );
        assert_eq!(
            render(ValueFormat::frequency_with_digits(1), 440.26),
            Ok(String::from("440.3 Hz"))
        );
    }

    #[test]
    fn frequency_uses_two_fractional_digits_by_default() {
        let policy = ValueFormat::frequency();

        assert_eq!(policy.fraction_digits(), DEFAULT_FREQUENCY_FRACTION_DIGITS);
        assert_eq!(render(policy, 1.0), Ok(String::from("1.00 Hz")));
    }

    #[test]
    fn changes_only_the_decimal_separator() {
        let policy = ValueFormat::percent(2).with_decimal_separator(DecimalSeparator::Comma);

        assert_eq!(render(policy, 0.125), Ok(String::from("12,50%")));
        assert_eq!(
            render(
                policy.with_decimal_separator(DecimalSeparator::Period),
                0.125
            ),
            Ok(String::from("12.50%"))
        );
    }

    #[test]
    fn rejects_nonfinite_values_before_writing() {
        for value in [f32::NAN, f32::NEG_INFINITY, f32::INFINITY] {
            let mut output = String::from("unchanged");

            assert_eq!(
                ValueFormat::decimal(2).write_into(value, &mut output),
                Err(ValueFormatError::NonFiniteValue)
            );
            assert_eq!(output, "unchanged");
        }
    }

    struct FailingWriter;

    impl fmt::Write for FailingWriter {
        fn write_str(&mut self, _value: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    #[test]
    fn maps_writer_failure_to_typed_error() {
        let mut output = FailingWriter;

        assert_eq!(
            ValueFormat::decimal(2).write_into(1.0, &mut output),
            Err(ValueFormatError::WriteFailed)
        );
    }

    #[test]
    fn caps_requested_precision_at_the_bounded_maximum() {
        let policy = ValueFormat::decimal(u8::MAX);

        assert_eq!(policy.fraction_digits(), ValueFormat::MAX_FRACTION_DIGITS);
        assert_eq!(render(policy, 1.0), Ok(String::from("1.000000000")));
    }

    #[test]
    fn policy_types_are_copy_and_accessors_expose_the_definition() {
        fn assert_copy<T: Copy>() {}
        fn assert_debug<T: fmt::Debug>() {}
        fn assert_eq<T: Eq>() {}

        assert_copy::<ValueFormat>();
        assert_copy::<ValueFormatKind>();
        assert_copy::<DecimalSeparator>();
        assert_copy::<ValueFormatError>();
        assert_debug::<ValueFormat>();
        assert_eq::<ValueFormat>();

        let policy =
            ValueFormat::frequency_with_digits(4).with_decimal_separator(DecimalSeparator::Comma);
        let copied = policy;

        assert_eq!(copied, policy);
        assert_eq!(policy.kind(), ValueFormatKind::Frequency);
        assert_eq!(policy.fraction_digits(), 4);
        assert_eq!(policy.decimal_separator(), DecimalSeparator::Comma);
    }
}
