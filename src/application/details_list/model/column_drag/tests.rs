use super::*;

#[test]
fn details_column_resize_drag_clamps_width() {
    let drag = DetailsColumnResizeDrag::new("name", 100.0, 240.0);

    assert_eq!(drag.width_at(130.0, 48.0, 420.0), 270.0);
    assert_eq!(drag.width_at(-500.0, 48.0, 420.0), 48.0);
    assert_eq!(drag.width_at(500.0, 48.0, 420.0), 420.0);
}

#[test]
fn details_column_reorder_drag_uses_estimated_content_left() {
    let placements = vec![
        DetailsColumnPlacement::new("name", 240.0),
        DetailsColumnPlacement::new("rating", 68.0),
        DetailsColumnPlacement::new("extension", 54.0),
    ];

    let content_left = details_column_drag_content_left(&placements, "rating", 300.0, 10.0)
        .expect("rating column should be found");
    let drag = DetailsColumnReorderDrag::new("rating", content_left);

    assert_eq!(content_left, 16.0);
    assert_eq!(drag.target_index(&placements, 410.0, 10.0), Some(2));
}
