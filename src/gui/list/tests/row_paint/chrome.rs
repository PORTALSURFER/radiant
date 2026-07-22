use super::fixtures::*;

#[test]
fn dense_row_chrome_parts_conditionally_adds_markers_and_outline() {
    let marker = DenseRowMarkerStyle::new(DenseRowMarkerParts::leading(2.0), SELECTED);
    let outline = DenseRowOutlineStyle::new(0.5, ACTIVE, 1.0);

    let enabled = DenseRowChromeParts::new(DenseRowVisualState::default(), palette())
        .leading_marker_if(true, marker)
        .leading_overlay_marker_if(true, marker)
        .trailing_marker_if(true, marker)
        .outline_if(true, outline);
    assert_eq!(enabled.leading_marker, Some(marker));
    assert_eq!(enabled.leading_overlay_marker, Some(marker));
    assert_eq!(enabled.trailing_marker, Some(marker));
    assert_eq!(enabled.outline, Some(outline));

    let disabled = DenseRowChromeParts::new(DenseRowVisualState::default(), palette())
        .leading_marker_if(false, marker)
        .leading_overlay_marker_if(false, marker)
        .trailing_marker_if(false, marker)
        .outline_if(false, outline);
    assert_eq!(disabled.leading_marker, None);
    assert_eq!(disabled.leading_overlay_marker, None);
    assert_eq!(disabled.trailing_marker, None);
    assert_eq!(disabled.outline, None);
}

#[test]
fn push_dense_row_chrome_composes_fill_markers_and_outline() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let mut primitives = Vec::new();

    let count = push_dense_row_chrome(
        &mut primitives,
        8,
        bounds,
        DenseRowChromeParts::new(
            DenseRowVisualState {
                selected: true,
                hovered: true,
                ..DenseRowVisualState::default()
            },
            palette(),
        )
        .leading_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
            SELECTED,
        ))
        .trailing_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::trailing(2.0),
            CANDIDATE,
        ))
        .outline(DenseRowOutlineStyle::new(0.5, ACTIVE, 1.0)),
    );

    assert_eq!(count, 4);
    assert_eq!(primitives.len(), 4);
    match &primitives[0] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(fill.rect, bounds);
            assert_eq!(fill.color, SELECTED_HOVERED);
        }
        primitive => panic!("expected row fill rect, got {primitive:?}"),
    }
    match &primitives[1] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(
                fill.rect,
                Rect::from_min_size(Point::new(11.0, 24.0), Vector2::new(3.0, 14.0))
            );
            assert_eq!(fill.color, SELECTED);
        }
        primitive => panic!("expected leading marker fill rect, got {primitive:?}"),
    }
    match &primitives[2] {
        PaintPrimitive::FillRect(fill) => {
            assert_eq!(
                fill.rect,
                Rect::from_min_size(Point::new(127.0, 23.0), Vector2::new(2.0, 16.0))
            );
            assert_eq!(fill.color, CANDIDATE);
        }
        primitive => panic!("expected trailing marker fill rect, got {primitive:?}"),
    }
    assert_eq!(
        primitives[3],
        PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: 8,
            rect: Rect::from_min_max(Point::new(10.5, 20.5), Point::new(129.5, 41.5)),
            color: ACTIVE,
            width: 1.0,
        })
    );
}

#[test]
fn push_dense_row_labeled_chrome_appends_chrome_before_label() {
    let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(120.0, 22.0));
    let mut primitives = Vec::new();
    let chrome = DenseRowChromeParts::new(
        DenseRowVisualState {
            hovered: true,
            ..DenseRowVisualState::default()
        },
        DenseRowPalette::new().hovered(HOVERED),
    )
    .leading_marker(DenseRowMarkerStyle::new(
        DenseRowMarkerParts::leading(2.0),
        SELECTED,
    ));

    let count = push_dense_row_labeled_chrome(
        &mut primitives,
        11,
        bounds,
        chrome,
        DenseRowLabelParts::new("Folder", PRESSED),
    );

    assert_eq!(count, 3);
    assert_eq!(primitives.len(), 3);
    assert!(matches!(
        &primitives[0],
        PaintPrimitive::FillRect(fill)
            if fill.widget_id == 11 && fill.rect == bounds && fill.color == HOVERED
    ));
    assert!(matches!(
        &primitives[1],
        PaintPrimitive::FillRect(fill) if fill.widget_id == 11 && fill.color == SELECTED
    ));
    assert!(matches!(
        &primitives[2],
        PaintPrimitive::Text(text)
            if text.widget_id == 11 && text.text == "Folder" && text.color == PRESSED
    ));
}
