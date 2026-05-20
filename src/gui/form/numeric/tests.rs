use super::{
    DecimalTextInputPolicy, parse_finite_decimal_text, rounded_scaled_u16,
    sanitize_decimal_text_insert,
};

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
