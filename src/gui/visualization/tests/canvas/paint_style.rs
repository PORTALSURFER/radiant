use super::fixtures::*;

#[test]
fn canvas_selection_geometry_pushes_common_affordance_fills() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    let mut primitives = Vec::<PaintPrimitive>::new();
    let color = Rgba8::new(1, 2, 3, 4);

    assert!(geometry.push_body_handle_fill(
        &mut primitives,
        42,
        CanvasSelectionBodyHandlePaintParts::new(7.0, 9.0, 0.28, 1.0, color),
    ));
    assert!(geometry.push_trailing_control_fill(
        &mut primitives,
        42,
        CanvasSelectionTrailingControlPaintParts::new(16.0, 0.0, color),
    ));
    assert!(geometry.push_edge_visual_fill(
        &mut primitives,
        42,
        CanvasSelectionEdgeVisualPaintParts::new(
            bounds.top_edge_strip(22.0),
            DragHandleRole::End,
            7.0,
            0.0,
            color,
        ),
    ));

    let fills = primitives
        .iter()
        .map(|primitive| primitive.fill_rect().expect("fill rect"))
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 3);
    assert_eq!(fills[0].widget_id, 42);
    assert_eq!(
        fills[0].rect,
        Rect::from_min_max(Point::new(59.0, 20.0), Point::new(121.0, 27.0))
    );
    assert_eq!(
        fills[1].rect,
        Rect::from_min_max(Point::new(114.0, 104.0), Point::new(130.0, 120.0))
    );
    assert_eq!(
        fills[2].rect,
        Rect::from_min_max(Point::new(126.5, 20.0), Point::new(133.5, 42.0))
    );
}

#[test]
fn canvas_selection_affordance_style_pushes_group_fills() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let edge_bounds = bounds.top_edge_strip(22.0);
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    let body_color = Rgba8::new(1, 2, 3, 4);
    let edge_color = Rgba8::new(5, 6, 7, 8);
    let trailing_color = Rgba8::new(9, 10, 11, 12);
    let style = CanvasSelectionAffordanceStyle::new()
        .with_body(CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0))
        .with_edge(CanvasSelectionEdgeVisualStyle::new(7.0, 0.0))
        .with_trailing_control(CanvasSelectionTrailingControlStyle::new(16.0, 0.0));
    let mut primitives = Vec::new();

    assert_eq!(
        style.push_fills(
            &mut primitives,
            42,
            geometry,
            CanvasSelectionAffordancePaintParts::new(edge_bounds)
                .body_color(body_color)
                .edge_color(edge_color)
                .trailing_control_color(trailing_color),
        ),
        4
    );

    let fills = primitives
        .iter()
        .map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => fill,
            primitive => panic!("expected fill rect, got {primitive:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(fills.len(), 4);
    assert_eq!(fills[0].widget_id, 42);
    assert_eq!(fills[0].color, edge_color);
    assert_eq!(fills[0].rect.min.x, 46.5);
    assert_eq!(fills[1].color, edge_color);
    assert_eq!(fills[1].rect.min.x, 126.5);
    assert_eq!(fills[2].color, body_color);
    assert_eq!(
        fills[2].rect,
        Rect::from_min_max(Point::new(59.0, 20.0), Point::new(121.0, 27.0))
    );
    assert_eq!(fills[3].color, trailing_color);
    assert_eq!(
        fills[3].rect,
        Rect::from_min_max(Point::new(114.0, 104.0), Point::new(130.0, 120.0))
    );
}

#[test]
fn canvas_selection_paint_style_derives_standard_colors() {
    let style = CanvasSelectionPaintStyle::new(Rgba8::new(20, 40, 60, 255))
        .fill_alpha(72)
        .cursor_alpha(210)
        .body_alpha(180)
        .edge_alpha(220)
        .trailing_control_alpha(230);

    assert_eq!(style.fill_color(), Rgba8::new(20, 40, 60, 72));
    assert_eq!(style.cursor_color(), Rgba8::new(20, 40, 60, 210));
    assert_eq!(style.body_color(), Rgba8::new(20, 40, 60, 180));
    assert_eq!(style.edge_color(), Rgba8::new(20, 40, 60, 220));
    assert_eq!(style.trailing_control_color(), Rgba8::new(20, 40, 60, 230));
}

#[test]
fn canvas_selection_paint_style_builds_affordance_paint_parts() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let edge_bounds = bounds.top_edge_strip(22.0);
    let parts = CanvasSelectionPaintStyle::new(Rgba8::new(90, 120, 240, 255))
        .body_alpha(180)
        .edge_alpha(220)
        .trailing_control_alpha(235)
        .affordance_paint_parts(edge_bounds);

    assert_eq!(parts.edge_bounds, edge_bounds);
    assert_eq!(parts.body_color, Some(Rgba8::new(90, 120, 240, 180)));
    assert_eq!(parts.edge_color, Some(Rgba8::new(90, 120, 240, 220)));
    assert_eq!(
        parts.trailing_control_color,
        Some(Rgba8::new(90, 120, 240, 235))
    );
}
