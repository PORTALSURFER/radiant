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

#[cfg(test)]
#[path = "numeric/tests.rs"]
mod tests;
