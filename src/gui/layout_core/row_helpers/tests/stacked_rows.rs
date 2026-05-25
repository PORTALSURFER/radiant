use super::*;

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
