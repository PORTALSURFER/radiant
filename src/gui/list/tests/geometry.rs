use crate::gui::list::{bounded_list_height, bounded_list_visible_rows};

#[test]
fn bounded_list_visible_rows_caps_to_configured_limit() {
    assert_eq!(bounded_list_visible_rows(0, 6), 0);
    assert_eq!(bounded_list_visible_rows(4, 6), 4);
    assert_eq!(bounded_list_visible_rows(12, 6), 6);
    assert_eq!(bounded_list_visible_rows(12, 0), 0);
}

#[test]
fn bounded_list_height_hides_empty_lists_and_caps_visible_rows() {
    assert_eq!(bounded_list_height(0, 6, 18.0, 6.0), 0.0);
    assert_eq!(bounded_list_height(4, 6, 18.0, 6.0), 78.0);
    assert_eq!(bounded_list_height(12, 6, 18.0, 6.0), 114.0);
    assert_eq!(bounded_list_height(12, 0, 18.0, 6.0), 0.0);
}

#[test]
fn bounded_list_height_normalizes_invalid_metrics() {
    assert_eq!(bounded_list_height(2, 6, -18.0, -6.0), 0.0);
    assert_eq!(bounded_list_height(2, 6, f32::NAN, 6.0), 6.0);
    assert_eq!(bounded_list_height(2, 6, 18.0, f32::INFINITY), 36.0);
}
