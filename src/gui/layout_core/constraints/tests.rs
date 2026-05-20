use super::Constraints;

#[test]
fn constraints_normalize_invalid_ranges() {
    let normalized = Constraints::new(-10.0, 4.0, 8.0, 2.0);
    assert_eq!(normalized.min_w, 0.0);
    assert_eq!(normalized.max_w, 4.0);
    assert_eq!(normalized.min_h, 8.0);
    assert_eq!(normalized.max_h, 8.0);
}
