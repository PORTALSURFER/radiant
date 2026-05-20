use super::{NormalizedRange, NormalizedRangeParts};

#[test]
fn normalized_range_orders_and_clamps_nano_bounds() {
    let range = NormalizedRange::from_nanos(1_200_000_000, 125_600_000);

    assert_eq!(range.start_nanos, 125_600_000);
    assert_eq!(range.end_nanos, 1_000_000_000);
    assert_eq!(range.start_micros, 125_600);
    assert_eq!(range.end_micros, 1_000_000);
    assert_eq!(range.start_milli, 126);
    assert_eq!(range.end_milli, 1000);
}

#[test]
fn normalized_range_from_milli_preserves_mirror_fields() {
    let range = NormalizedRange::new(800, 200);

    assert_eq!(range.start_milli, 200);
    assert_eq!(range.end_milli, 800);
    assert_eq!(range.start_micros, 200_000);
    assert_eq!(range.end_micros, 800_000);
    assert_eq!(range.start_nanos, 200_000_000);
    assert_eq!(range.end_nanos, 800_000_000);
}

#[test]
fn normalized_range_supports_named_parts_construction() {
    let range = NormalizedRange::from_parts(NormalizedRangeParts {
        start_milli: 1_200,
        end_milli: 250,
    });

    assert_eq!(range.start_milli, 250);
    assert_eq!(range.end_milli, 1000);
    assert_eq!(range.start_micros, 250_000);
    assert_eq!(range.end_micros, 1_000_000);
}
