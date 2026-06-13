use super::fixtures::*;

#[test]
fn push_dense_row_label_appends_centered_text_run() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let mut primitives = Vec::new();

    assert!(push_dense_row_label(
        &mut primitives,
        9,
        bounds,
        DenseRowLabelParts::new("Folder", SELECTED).inset_x(6.0),
    ));

    assert_eq!(primitives.len(), 1);
    match &primitives[0] {
        PaintPrimitive::Text(text) => {
            assert_eq!(text.widget_id, 9);
            assert_eq!(text.text, "Folder");
            assert_eq!(text.color, SELECTED);
            assert_eq!(text.font_size, 13.0);
            assert!((text.rect.min.x - 16.0).abs() < 0.01, "{:?}", text.rect);
        }
        primitive => panic!("expected text run, got {primitive:?}"),
    }
}

#[test]
fn push_dense_row_label_skips_empty_or_collapsed_rows() {
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 22.0));
    let collapsed = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 22.0));
    let mut primitives = Vec::new();

    assert!(!push_dense_row_label(
        &mut primitives,
        9,
        bounds,
        DenseRowLabelParts::new("", SELECTED),
    ));
    assert!(!push_dense_row_label(
        &mut primitives,
        9,
        collapsed,
        DenseRowLabelParts::new("Folder", SELECTED),
    ));
    assert!(primitives.is_empty());
}
