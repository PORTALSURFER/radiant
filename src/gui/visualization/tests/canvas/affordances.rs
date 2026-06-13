use super::fixtures::*;

#[test]
fn canvas_selection_affordance_hit_test_uses_editor_priority_order() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    let trailing_over_body = CanvasSelectionAffordanceHitTestParts::new()
        .with_body(CanvasSelectionBodyHandleHitTestParts::new(
            Point::new(120.0, 110.0),
            100.0,
            0.0,
            0.0,
            1.0,
        ))
        .with_trailing_control(CanvasSelectionTrailingControlHitTestParts::new(
            Point::new(120.0, 110.0),
            16.0,
            0.0,
        ));

    assert_eq!(
        geometry.affordance_at_point(trailing_over_body),
        Some(DragHandleRole::TrailingControl)
    );

    let edge_over_body = CanvasSelectionAffordanceHitTestParts::new()
        .with_body(CanvasSelectionBodyHandleHitTestParts::new(
            Point::new(130.0, 24.0),
            90.0,
            0.0,
            0.0,
            1.0,
        ))
        .with_edge(CanvasSelectionEdgeHitTestParts::new(
            bounds.top_edge_strip(22.0),
            Point::new(130.0, 24.0),
            7.0,
            0.0,
        ));

    assert_eq!(
        geometry.affordance_at_point(edge_over_body),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts {
            edge: None,
            ..edge_over_body
        }),
        Some(DragHandleRole::Body)
    );
}

#[test]
fn canvas_selection_affordance_hit_test_can_check_single_groups() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_edge(
            CanvasSelectionEdgeHitTestParts::new(
                bounds.top_edge_strip(22.0),
                Point::new(50.0, 24.0),
                7.0,
                0.0,
            )
        )),
        Some(DragHandleRole::Start)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_body(
            CanvasSelectionBodyHandleHitTestParts::new(Point::new(60.0, 24.0), 7.0, 9.0, 0.28, 1.0,)
        )),
        Some(DragHandleRole::Body)
    );
    assert_eq!(
        geometry.affordance_at_point(
            CanvasSelectionAffordanceHitTestParts::new().with_trailing_control(
                CanvasSelectionTrailingControlHitTestParts::new(
                    Point::new(120.0, 110.0),
                    16.0,
                    0.0,
                )
            )
        ),
        Some(DragHandleRole::TrailingControl)
    );
    assert_eq!(
        geometry.affordance_at_point(CanvasSelectionAffordanceHitTestParts::new().with_body(
            CanvasSelectionBodyHandleHitTestParts::new(Point::new(60.0, 40.0), 7.0, 9.0, 0.28, 1.0,)
        )),
        None
    );
}

#[test]
fn canvas_selection_affordance_styles_build_matching_hit_test_and_paint_parts() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let point = Point::new(60.0, 24.0);
    let color = Rgba8::new(1, 2, 3, 4);

    let body = CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0);
    assert_eq!(
        body.hit_test_parts(point),
        CanvasSelectionBodyHandleHitTestParts::new(point, 7.0, 9.0, 0.28, 1.0)
    );
    assert_eq!(
        body.paint_parts(color),
        CanvasSelectionBodyHandlePaintParts::new(7.0, 9.0, 0.28, 1.0, color)
    );

    let edge = CanvasSelectionEdgeVisualStyle::new(7.0, 0.0);
    assert_eq!(
        edge.hit_test_parts(bounds.top_edge_strip(22.0), point),
        CanvasSelectionEdgeHitTestParts::new(bounds.top_edge_strip(22.0), point, 7.0, 0.0)
    );
    assert_eq!(
        edge.paint_parts(bounds.top_edge_strip(22.0), DragHandleRole::End, color),
        CanvasSelectionEdgeVisualPaintParts::new(
            bounds.top_edge_strip(22.0),
            DragHandleRole::End,
            7.0,
            0.0,
            color,
        )
    );

    let trailing = CanvasSelectionTrailingControlStyle::new(16.0, 0.0);
    assert_eq!(
        trailing.hit_test_parts(point),
        CanvasSelectionTrailingControlHitTestParts::new(point, 16.0, 0.0)
    );
    assert_eq!(
        trailing.paint_parts(color),
        CanvasSelectionTrailingControlPaintParts::new(16.0, 0.0, color)
    );
}

#[test]
fn canvas_selection_affordance_style_builds_group_hit_tests() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let point = Point::new(130.0, 24.0);

    let style = CanvasSelectionAffordanceStyle::new()
        .with_body(CanvasSelectionBodyHandleStyle::new(7.0, 9.0, 0.28, 1.0))
        .with_edge(CanvasSelectionEdgeVisualStyle::new(7.0, 0.0))
        .with_trailing_control(CanvasSelectionTrailingControlStyle::new(16.0, 0.0));

    assert_eq!(
        style.hit_test_parts(bounds.top_edge_strip(22.0), point),
        CanvasSelectionAffordanceHitTestParts::new()
            .with_body(CanvasSelectionBodyHandleHitTestParts::new(
                point, 7.0, 9.0, 0.28, 1.0
            ))
            .with_edge(CanvasSelectionEdgeHitTestParts::new(
                bounds.top_edge_strip(22.0),
                point,
                7.0,
                0.0,
            ))
            .with_trailing_control(CanvasSelectionTrailingControlHitTestParts::new(
                point, 16.0, 0.0,
            ))
    );

    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");
    assert_eq!(
        style.affordance_at_point(geometry, bounds.top_edge_strip(22.0), point),
        Some(DragHandleRole::End)
    );
}
