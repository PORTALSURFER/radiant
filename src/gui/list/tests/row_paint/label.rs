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

#[test]
fn environment_label_uses_declared_metrics_for_clipped_and_expanded_bounds() {
    for (scale, expected_font, expected_inset) in
        [(1.0, 13.0, 4.0), (1.5, 19.5, 6.0), (2.0, 26.0, 8.0)]
    {
        for height in [11.0, 60.0] {
            let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, height));
            let mut primitives = Vec::new();
            assert!(push_dense_row_label_with_environment(
                &mut primitives,
                19,
                bounds,
                DenseRowLabelParts::new("Folder", SELECTED).offset_y(2.0),
                declared_metrics(),
                &environment(scale, 2.0),
            ));
            let PaintPrimitive::Text(text) = &primitives[0] else {
                panic!("expected text run");
            };
            assert_eq!(text.font_size, expected_font);
            assert_eq!(text.rect.min.x, bounds.min.x + expected_inset);
        }
    }
}

#[test]
fn environment_label_metrics_ignore_dpi_and_resolve_offset_once() {
    for scale in [1.0, 1.5, 2.0] {
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 11.0));
        let mut one = Vec::new();
        let mut two = Vec::new();
        push_dense_row_label_with_environment(
            &mut one,
            20,
            bounds,
            DenseRowLabelParts::new("Folder", SELECTED).offset_y(0.0),
            declared_metrics(),
            &environment(scale, 1.0),
        );
        push_dense_row_label_with_environment(
            &mut two,
            20,
            bounds,
            DenseRowLabelParts::new("Folder", SELECTED).offset_y(2.0),
            declared_metrics(),
            &environment(scale, 2.0),
        );
        let PaintPrimitive::Text(first) = &one[0] else {
            panic!("expected text run");
        };
        let PaintPrimitive::Text(second) = &two[0] else {
            panic!("expected text run");
        };
        assert_eq!(first.font_size, second.font_size);
        assert_eq!(first.rect.min.x, second.rect.min.x);
        assert!((second.rect.min.y - first.rect.min.y - 2.0 * scale).abs() < 0.001);
    }
}

#[test]
fn environment_label_repeated_frames_reproject_from_the_declaration() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 60.0));
    let mut observed = Vec::new();
    for scale in [1.0, 1.5, 2.0, 1.0] {
        let mut primitives = Vec::new();
        push_dense_row_label_with_environment(
            &mut primitives,
            21,
            bounds,
            DenseRowLabelParts::new("Folder", SELECTED),
            declared_metrics(),
            &environment(scale, 1.0),
        );
        let PaintPrimitive::Text(text) = &primitives[0] else {
            panic!("expected text run");
        };
        observed.push((text.font_size, text.rect.min.x));
    }
    assert_eq!(
        observed,
        [(13.0, 4.0), (19.5, 6.0), (26.0, 8.0), (13.0, 4.0)]
    );
}
