use super::*;

#[test]
fn grouped_fixed_width_row_width_counts_visible_groups_and_gaps() {
    assert_eq!(fixed_width_group_width(10.0, 3, 2.0), 34.0);
    assert_eq!(
        grouped_fixed_width_row_width(10.0, &[3, 0, 2], 2.0, 6.0),
        62.0
    );
    assert_eq!(grouped_fixed_width_row_width(0.0, &[3], 2.0, 6.0), 0.0);
    assert_eq!(grouped_fixed_width_row_width(10.0, &[], 2.0, 6.0), 0.0);
}
