//! Generic form and picker primitives.

/// Policy for a decimal numeric single-line text field.
///
/// The policy is intentionally domain-neutral: callers decide what the parsed
/// value means and which action/message should be emitted after a valid edit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecimalTextInputPolicy {
    /// Whether zero and negative values should be rejected by `parse_value`.
    pub positive_only: bool,
}

impl DecimalTextInputPolicy {
    /// Decimal field policy that accepts any finite `f32` value.
    pub const FINITE: Self = Self {
        positive_only: false,
    };

    /// Decimal field policy that accepts only positive finite `f32` values.
    pub const POSITIVE_FINITE: Self = Self {
        positive_only: true,
    };

    /// Sanitize inserted text for a decimal field.
    ///
    /// The existing `selection_range` must be a valid byte range in `current`.
    /// Inserted text keeps ASCII digits and at most one decimal separator,
    /// preserving a decimal point that already exists outside the selection.
    pub fn sanitize_insert(
        self,
        current: &str,
        selection_range: (usize, usize),
        inserted: &str,
    ) -> String {
        sanitize_decimal_text_insert(current, selection_range, inserted)
    }

    /// Parse a finite `f32` value according to this field policy.
    pub fn parse_value(self, text: &str) -> Option<f32> {
        let parsed = parse_finite_decimal_text(text)?;
        if self.positive_only && parsed <= 0.0 {
            return None;
        }
        Some(parsed)
    }
}

/// Sanitize inserted text so a decimal numeric field only accepts digits and
/// one decimal separator.
///
/// The existing `selection_range` must be a valid byte range in `current`.
pub fn sanitize_decimal_text_insert(
    current: &str,
    selection_range: (usize, usize),
    inserted: &str,
) -> String {
    let (selection_start, selection_end) = selection_range;
    let mut sanitized = String::with_capacity(inserted.len());
    let mut has_decimal =
        current[..selection_start].contains('.') || current[selection_end..].contains('.');
    for ch in inserted.chars() {
        if ch.is_ascii_digit() {
            sanitized.push(ch);
        } else if ch == '.' && !has_decimal {
            sanitized.push(ch);
            has_decimal = true;
        }
    }
    sanitized
}

/// Parse a finite decimal value from a single-line text field.
pub fn parse_finite_decimal_text(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = trimmed.parse::<f32>().ok()?;
    parsed.is_finite().then_some(parsed)
}

/// Convert a finite numeric field value into a rounded, clamped `u16` scale.
pub fn rounded_scaled_u16(value: f32, scale: f32) -> u16 {
    if !value.is_finite() || !scale.is_finite() {
        return 0;
    }
    let scaled = value * scale;
    if !scaled.is_finite() {
        return if scaled.is_sign_positive() {
            u16::MAX
        } else {
            0
        };
    }
    scaled.round().clamp(0.0, u16::MAX as f32) as u16
}

/// One selectable option in a picker, menu, or segmented choice list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionItem<Value> {
    /// Human-readable option label.
    pub label: String,
    /// Whether the option is currently selected.
    pub selected: bool,
    /// Value applied when the option is chosen.
    pub value: Value,
}

/// Overview row for a labeled field and its current value summary.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SummaryField {
    /// Static row label.
    pub label: String,
    /// Current value summary.
    pub value_label: String,
}

/// Field currently expanded inside a paired picker surface.
///
/// Paired pickers are useful for option panels that expose the same group/item/
/// numeric controls for two related sides, such as primary/secondary endpoints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairedPickerTarget {
    /// Group picker for the primary side.
    PrimaryGroup,
    /// Item picker for the primary side.
    PrimaryItem,
    /// Numeric picker for the primary side.
    PrimaryNumber,
    /// Group picker for the secondary side.
    SecondaryGroup,
    /// Item picker for the secondary side.
    SecondaryItem,
    /// Numeric picker for the secondary side.
    SecondaryNumber,
}

/// Raw value carried by one paired-picker option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairedPickerValue<Text = String, Number = u32> {
    /// Primary-side group value, or `None` for an inherited/default value.
    PrimaryGroup(Option<Text>),
    /// Primary-side item value, or `None` for an inherited/default value.
    PrimaryItem(Option<Text>),
    /// Primary-side numeric value, or `None` for an inherited/default value.
    PrimaryNumber(Option<Number>),
    /// Secondary-side group value, or `None` for an inherited/default value.
    SecondaryGroup(Option<Text>),
    /// Secondary-side item value, or `None` for an inherited/default value.
    SecondaryItem(Option<Text>),
    /// Secondary-side numeric value, or `None` for an inherited/default value.
    SecondaryNumber(Option<Number>),
}

#[cfg(test)]
mod tests {
    use super::{
        DecimalTextInputPolicy, OptionItem, PairedPickerTarget, PairedPickerValue, SummaryField,
        parse_finite_decimal_text, rounded_scaled_u16, sanitize_decimal_text_insert,
    };

    #[test]
    fn option_item_preserves_label_selection_and_value() {
        let option = OptionItem {
            label: String::from("Default"),
            selected: true,
            value: Some(48_000_u32),
        };

        assert_eq!(option.label, "Default");
        assert!(option.selected);
        assert_eq!(option.value, Some(48_000));
    }

    #[test]
    fn summary_field_defaults_to_empty_text() {
        let field = SummaryField::default();

        assert_eq!(field.label, "");
        assert_eq!(field.value_label, "");
    }

    #[test]
    fn paired_picker_models_cover_primary_and_secondary_fields() {
        let target = PairedPickerTarget::SecondaryNumber;
        let value: PairedPickerValue<String, u32> = PairedPickerValue::PrimaryNumber(Some(48_000));

        assert_eq!(target, PairedPickerTarget::SecondaryNumber);
        assert_eq!(value, PairedPickerValue::PrimaryNumber(Some(48_000)));
    }

    #[test]
    fn decimal_text_insert_keeps_digits_and_one_decimal_point() {
        assert_eq!(sanitize_decimal_text_insert("12.3", (0, 0), "a4.5"), "45");
        assert_eq!(sanitize_decimal_text_insert("123", (1, 2), "a4.5"), "4.5");
    }

    #[test]
    fn decimal_text_policy_parses_finite_values_with_optional_positive_gate() {
        assert_eq!(parse_finite_decimal_text(" 12.5 "), Some(12.5));
        assert_eq!(parse_finite_decimal_text(""), None);
        assert_eq!(parse_finite_decimal_text("NaN"), None);
        assert_eq!(DecimalTextInputPolicy::FINITE.parse_value("-1"), Some(-1.0));
        assert_eq!(
            DecimalTextInputPolicy::POSITIVE_FINITE.parse_value("-1"),
            None
        );
        assert_eq!(
            DecimalTextInputPolicy::POSITIVE_FINITE.parse_value("0"),
            None
        );
        assert_eq!(
            DecimalTextInputPolicy::POSITIVE_FINITE.parse_value("1"),
            Some(1.0)
        );
    }

    #[test]
    fn rounded_scaled_u16_clamps_non_finite_and_large_values() {
        assert_eq!(rounded_scaled_u16(12.34, 10.0), 123);
        assert_eq!(rounded_scaled_u16(f32::NAN, 10.0), 0);
        assert_eq!(rounded_scaled_u16(f32::MAX, 10.0), u16::MAX);
    }
}
