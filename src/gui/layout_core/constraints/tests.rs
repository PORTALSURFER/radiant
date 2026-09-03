use super::Constraints;

#[test]
fn constraints_normalize_invalid_ranges() {
    let normalized = Constraints::new(-10.0, 4.0, 8.0, 2.0);
    assert_eq!(normalized.min_w, 0.0);
    assert_eq!(normalized.max_w, 4.0);
    assert_eq!(normalized.min_h, 8.0);
    assert_eq!(normalized.max_h, 8.0);
}

#[test]
fn direct_constraints_do_not_retain_nonfinite_requested_maxima() {
    for maximum in [f32::NAN, f32::NEG_INFINITY] {
        let normalized = Constraints {
            min_w: 4.0,
            max_w: maximum,
            min_h: 6.0,
            max_h: maximum,
        }
        .normalized();

        assert_eq!(normalized.max_w, normalized.min_w);
        assert_eq!(normalized.max_h, normalized.min_h);
        assert!(normalized.max_w.is_finite());
        assert!(normalized.max_h.is_finite());
    }
}

#[test]
fn inset_saturates_overflowing_negative_finite_insets() {
    let inset = Constraints {
        min_w: 0.0,
        max_w: 100.0,
        min_h: 0.0,
        max_h: 80.0,
    }
    .inset(-f32::MAX, -f32::MAX);

    assert_eq!(inset.max_w, f32::MAX);
    assert_eq!(inset.max_h, f32::MAX);
    assert!(inset.max_w.is_finite());
    assert!(inset.max_h.is_finite());
}
