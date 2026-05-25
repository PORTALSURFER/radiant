use super::*;

#[test]
fn visible_suffix_widths_preserves_rightmost_items_that_fit() {
    assert_eq!(
        visible_suffix_widths(&[20.0, 30.0, 40.0], 80.0, 4.0),
        [30.0, 40.0]
    );
    assert_eq!(visible_suffix_widths(&[20.0], 20.0, 4.0), [20.0]);
    assert!(visible_suffix_widths(&[20.0], 19.9, 4.0).is_empty());
    assert_eq!(visible_suffix_widths(&[20.0], 20.1, 4.0), [20.0]);
    assert_eq!(
        visible_suffix_widths(&[20.0, 30.0, 40.0], 74.0, 4.0),
        [30.0, 40.0]
    );
}

#[test]
fn visible_suffix_widths_normalizes_negative_dimensions() {
    assert_eq!(
        visible_suffix_widths(&[20.0, -30.0, 40.0], 60.0, -4.0),
        [20.0, 0.0, 40.0]
    );
    assert!(visible_suffix_widths(&[20.0, -30.0, 40.0], 39.9, -4.0).is_empty());
}

#[test]
fn visible_suffix_widths_into_reuses_output_storage() {
    let mut visible = Vec::with_capacity(8);
    visible.push(99.0);
    let capacity = visible.capacity();

    visible_suffix_widths_into(&[20.0, 30.0, 40.0], 80.0, 4.0, &mut visible);

    assert_eq!(visible, [30.0, 40.0]);
    assert_eq!(visible.capacity(), capacity);

    visible_suffix_widths_into(&[20.0, 30.0, 40.0], 0.0, 4.0, &mut visible);

    assert!(visible.is_empty());
    assert_eq!(visible.capacity(), capacity);
}

#[test]
fn fixed_width_item_extent_for_available_width_fits_items_after_reserved_gaps() {
    assert_eq!(
        fixed_width_item_extent_for_available_width(100.0, 4, 12.0, 6.0, 20.0),
        20.0
    );
    assert_eq!(
        fixed_width_item_extent_for_available_width(40.0, 4, 12.0, 6.0, 20.0),
        7.0
    );
    assert_eq!(
        fixed_width_item_extent_for_available_width(20.0, 4, 12.0, 6.0, 20.0),
        6.0
    );
    assert_eq!(
        fixed_width_item_extent_for_available_width(10.0, 4, 12.0, 6.0, 20.0),
        0.0
    );
    assert_eq!(
        fixed_width_item_extent_for_available_width(100.0, 4, 12.0, 30.0, 20.0),
        30.0
    );
}
