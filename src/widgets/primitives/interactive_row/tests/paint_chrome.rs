use super::*;

#[test]
fn push_dense_fill_uses_row_state_and_identity() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    let bounds = Rect::from_size(120.0, 22.0);
    let color = Rgba8::new(8, 9, 10, 180);
    let mut primitives = Vec::new();

    assert!(
        row.push_dense_fill(
            &mut primitives,
            bounds,
            InteractiveRowVisualStateParts {
                selected: true,
                ..InteractiveRowVisualStateParts::default()
            },
            DenseRowPalette::new()
                .selected(Rgba8::new(1, 2, 3, 120))
                .hovered(color),
        )
    );

    assert!(matches!(
        primitives.as_slice(),
        [PaintPrimitive::FillRect(fill)]
            if fill.widget_id == row.id() && fill.rect == bounds && fill.color == color
    ));
}

#[test]
fn push_dense_chrome_uses_row_state_and_identity() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    let bounds = Rect::from_size(120.0, 22.0);
    let hover = Rgba8::new(8, 9, 10, 180);
    let marker = Rgba8::new(220, 120, 60, 255);
    let mut primitives = Vec::new();
    let parts = row
        .dense_chrome_parts(
            InteractiveRowVisualStateParts {
                selected: true,
                ..InteractiveRowVisualStateParts::default()
            },
            DenseRowPalette::new().hovered(hover),
        )
        .leading_marker(DenseRowMarkerStyle::new(
            DenseRowMarkerParts::leading(2.0),
            marker,
        ));

    assert_eq!(row.push_dense_chrome(&mut primitives, bounds, parts), 2);
    assert!(matches!(
        primitives.as_slice(),
        [PaintPrimitive::FillRect(fill), PaintPrimitive::FillRect(marker_fill)]
            if fill.widget_id == row.id()
                && fill.rect == bounds
                && fill.color == hover
                && marker_fill.widget_id == row.id()
                && marker_fill.color == marker
    ));
}

#[test]
fn push_dense_labeled_chrome_uses_row_state_identity_and_label() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;
    let bounds = Rect::from_size(120.0, 22.0);
    let hover = Rgba8::new(8, 9, 10, 180);
    let label = Rgba8::new(180, 220, 255, 255);
    let mut primitives = Vec::new();
    let parts = row.dense_chrome_parts(
        InteractiveRowVisualStateParts {
            selected: true,
            ..InteractiveRowVisualStateParts::default()
        },
        DenseRowPalette::new().hovered(hover),
    );

    assert_eq!(
        row.push_dense_labeled_chrome(
            &mut primitives,
            bounds,
            parts,
            DenseRowLabelParts::new("Folder", label),
        ),
        2
    );
    assert!(matches!(
        primitives.as_slice(),
        [PaintPrimitive::FillRect(fill), PaintPrimitive::Text(text)]
            if fill.widget_id == row.id()
                && fill.rect == bounds
                && fill.color == hover
                && text.widget_id == row.id()
                && text.text == "Folder"
                && text.color == label
    ));
}

#[test]
fn paint_plan_with_defaults_exposes_query_helpers_for_one_widget() {
    let mut row = InteractiveRowWidget::new(7, WidgetSizing::fixed(Vector2::new(120.0, 22.0)));
    row.common.state.hovered = true;

    let plan = row.paint_plan_with_defaults(Rect::from_size(120.0, 22.0));

    assert_eq!(
        plan.fill_rects().next().map(|fill| fill.widget_id),
        Some(row.id())
    );
}
