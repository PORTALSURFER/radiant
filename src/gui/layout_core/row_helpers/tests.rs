use super::{
    StackedRowRectsParts, fixed_width_group_width, fixed_width_item_extent_for_available_width,
    fixed_width_row_rects_end, fixed_width_row_rects_end_into, fixed_width_row_rects_start,
    fixed_width_row_rects_start_into, grouped_fixed_width_row_width, stacked_row_rects,
    stacked_row_rects_from_parts, stacked_row_rects_into, stacked_row_rects_into_from_parts,
    visible_suffix_widths, visible_suffix_widths_into,
};
use crate::gui::types::{Point, Rect};

#[test]
fn fixed_width_row_rects_start_places_items_from_left_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let rects = fixed_width_row_rects_start(bounds, 4.0, &[20.0, 30.0], 1, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[0].max.x, 30.0);
    assert_eq!(rects[1].min.x, 34.0);
    assert_eq!(rects[1].max.x, 64.0);
}

#[test]
fn fixed_width_row_rects_end_places_items_against_right_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let rects = fixed_width_row_rects_end(bounds, 4.0, &[20.0, 30.0], 1, 2, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 56.0);
    assert_eq!(rects[0].max.x, 76.0);
    assert_eq!(rects[1].min.x, 80.0);
    assert_eq!(rects[1].max.x, 110.0);
}

#[test]
fn fixed_width_row_rects_end_overflow_starts_at_left_edge() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(60.0, 40.0));
    let rects = fixed_width_row_rects_end(bounds, 4.0, &[40.0, 40.0], 1, 2, 10);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[0].max.x, 50.0);
    assert_eq!(rects[1].min.x, 54.0);
    assert_eq!(rects[1].max.x, 60.0);
}

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
fn fixed_width_row_rects_presizes_output() {
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(120.0, 20.0));
    let rects = fixed_width_row_rects_start(bounds, 2.0, &[10.0, 20.0, 30.0], 1, 10);

    assert_eq!(rects.len(), 3);
    assert!(rects.capacity() >= 3);
}

#[test]
fn fixed_width_row_rects_into_reuses_output_storage() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(110.0, 40.0));
    let mut rects = Vec::with_capacity(8);
    rects.push(Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ));
    let capacity = rects.capacity();

    fixed_width_row_rects_start_into(bounds, 4.0, &[20.0, 30.0], 1, 10, &mut rects);

    assert_eq!(rects.len(), 2);
    assert_eq!(rects.capacity(), capacity);
    assert_eq!(rects[0].min.x, 10.0);
    assert_eq!(rects[1].max.x, 64.0);

    fixed_width_row_rects_end_into(bounds, 4.0, &[20.0, 30.0], 1, 2, 10, &mut rects);

    assert_eq!(rects.capacity(), capacity);
    assert_eq!(rects[0].min.x, 56.0);
    assert_eq!(rects[1].max.x, 110.0);
}

#[test]
fn stacked_row_rects_clamps_rows_to_column() {
    let column = Rect::from_min_max(Point::new(10.0, 20.2), Point::new(110.0, 76.4));
    let rows = stacked_row_rects_from_parts(StackedRowRectsParts {
        column,
        rows: 6,
        gap: 1.4,
        row_height: 15.8,
    });

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].min.y, 20.0);
    assert_eq!(rows[0].max.y, 36.0);
    assert_eq!(rows[3].min.y, 72.0);
    assert_eq!(rows[3].max.y, 76.0);
    assert!(
        rows.iter()
            .all(|row| row.min.x == 10.0 && row.max.x == 110.0)
    );
}

#[test]
fn stacked_row_rects_compatibility_helper_delegates_to_named_parts() {
    let column = Rect::from_min_max(Point::new(10.0, 20.2), Point::new(110.0, 76.4));
    let from_parts = stacked_row_rects_from_parts(StackedRowRectsParts {
        column,
        rows: 6,
        gap: 1.4,
        row_height: 15.8,
    });

    assert_eq!(stacked_row_rects(column, 6, 1.4, 15.8), from_parts);
}

#[test]
fn stacked_row_rects_into_reuses_output_storage() {
    let column = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(80.0, 80.0));
    let mut rows = Vec::with_capacity(8);
    rows.push(Rect::from_min_max(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ));
    let capacity = rows.capacity();

    stacked_row_rects_into_from_parts(
        StackedRowRectsParts {
            column,
            rows: 3,
            gap: 2.0,
            row_height: 10.0,
        },
        &mut rows,
    );

    assert_eq!(rows.len(), 3);
    assert_eq!(rows.capacity(), capacity);
    assert_eq!(rows[0].min.y, 0.0);
    assert_eq!(rows[2].min.y, 24.0);
}

#[test]
fn stacked_row_rects_into_compatibility_helper_delegates_to_named_parts() {
    let column = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(80.0, 80.0));
    let mut from_parts = Vec::new();
    let mut positional = Vec::new();

    stacked_row_rects_into_from_parts(
        StackedRowRectsParts {
            column,
            rows: 3,
            gap: 2.0,
            row_height: 10.0,
        },
        &mut from_parts,
    );
    stacked_row_rects_into(column, 3, 2.0, 10.0, &mut positional);

    assert_eq!(positional, from_parts);
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
fn grouped_fixed_width_row_width_counts_visible_groups_and_gaps() {
    assert_eq!(fixed_width_group_width(10.0, 3, 2.0), 34.0);
    assert_eq!(
        grouped_fixed_width_row_width(10.0, &[3, 0, 2], 2.0, 6.0),
        62.0
    );
    assert_eq!(grouped_fixed_width_row_width(0.0, &[3], 2.0, 6.0), 0.0);
    assert_eq!(grouped_fixed_width_row_width(10.0, &[], 2.0, 6.0), 0.0);
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
