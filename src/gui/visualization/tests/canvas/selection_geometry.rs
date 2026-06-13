use super::fixtures::*;

#[test]
fn canvas_selection_rect_projects_normalized_range() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_rect(bounds, 0.25, 0.75),
        Some(Rect::from_min_max(
            Point::new(60.0, 20.0),
            Point::new(160.0, 120.0)
        ))
    );
    assert_eq!(canvas_selection_rect(bounds, 0.75, 0.25), None);
    assert_eq!(
        canvas_selection_rect(bounds, f32::NAN, 0.5),
        Some(Rect::from_min_max(
            Point::new(10.0, 20.0),
            Point::new(110.0, 120.0)
        ))
    );
}

#[test]
fn canvas_selection_geometry_projects_common_affordances() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let geometry = CanvasSelectionGeometry::new(bounds, 0.2, 0.6).expect("geometry");

    assert_eq!(
        geometry.rect,
        Rect::from_min_max(Point::new(50.0, 20.0), Point::new(130.0, 120.0))
    );
    assert_eq!(
        geometry.body_handle_rect(7.0, 9.0, 0.28, 1.0),
        Some(Rect::from_min_max(
            Point::new(59.0, 20.0),
            Point::new(121.0, 27.0)
        ))
    );
    assert_eq!(
        geometry.trailing_control_rect(16.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(114.0, 104.0),
            Point::new(130.0, 120.0)
        ))
    );
    assert!(geometry.body_handle_at_point(7.0, 9.0, 0.28, 1.0, Point::new(60.0, 24.0)));
    assert!(!geometry.body_handle_at_point(7.0, 9.0, 0.28, 1.0, Point::new(60.0, 40.0)));
    assert!(geometry.trailing_control_at_point(16.0, 0.0, Point::new(120.0, 110.0)));
    assert!(!geometry.trailing_control_at_point(16.0, 0.0, Point::new(120.0, 90.0)));
    assert_eq!(
        geometry.edge_visual_rect(bounds.top_edge_strip(22.0), DragHandleRole::End, 7.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(126.5, 20.0),
            Point::new(133.5, 42.0)
        ))
    );
    assert_eq!(
        geometry.edge_at_point(
            bounds.top_edge_strip(22.0),
            Point::new(130.0, 24.0),
            7.0,
            0.0
        ),
        Some(DragHandleRole::End)
    );
}

#[test]
fn canvas_selection_geometry_projects_viewport_clipped_range() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let viewport = IndexViewportScope::new(
        IndexViewport {
            start: 200,
            end: 600,
        },
        1_000,
        10,
    );
    let geometry = CanvasSelectionGeometry::from_viewport_range(
        bounds,
        viewport,
        NormalizedRange::from_fractions(0.3, 0.7),
    )
    .expect("visible geometry");

    assert!((geometry.start_fraction - 0.25).abs() <= f32::EPSILON);
    assert!((geometry.end_fraction - 1.0).abs() <= f32::EPSILON);
    assert!((geometry.rect.min.x - 60.0).abs() <= 0.001);
    assert_eq!(geometry.rect.min.y, 20.0);
    assert_eq!(geometry.rect.max.x, 210.0);
    assert_eq!(geometry.rect.max.y, 120.0);
    assert_eq!(
        CanvasSelectionGeometry::from_viewport_range(
            bounds,
            viewport,
            NormalizedRange::from_fractions(0.7, 0.8),
        ),
        None
    );
}

#[test]
fn canvas_selection_edge_handles_project_hit_targets() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));
    let handles = canvas_selection_edge_handles(bounds, 0.25, 0.75, 18.0, 42).expect("handles");

    assert_eq!(handles[0].role, DragHandleRole::Start);
    assert_eq!(handles[0].capture_token, 42);
    assert_eq!(
        handles[0].rect,
        Rect::from_min_max(Point::new(51.0, 20.0), Point::new(69.0, 120.0))
    );
    assert_eq!(handles[1].role, DragHandleRole::End);
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(160.0, 60.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        canvas_selection_edge_handles(bounds, 0.25, 0.75, 0.0, 42),
        None
    );
}

#[test]
fn canvas_selection_edge_visual_rect_projects_inset_handle() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_edge_visual_rect(bounds, 0.25, 8.0, 16.0),
        Some(Rect::from_min_max(
            Point::new(56.0, 36.0),
            Point::new(64.0, 104.0)
        ))
    );
    assert_eq!(
        canvas_selection_edge_visual_rect(bounds, 0.25, 8.0, 60.0),
        None
    );
}

#[test]
fn canvas_selection_body_handle_rect_projects_inset_top_strip() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds,
            start_fraction: 0.2,
            end_fraction: 0.6,
            height: 7.0,
            end_inset: 9.0,
            max_end_inset_fraction: 0.28,
            min_width_after_inset: 1.0,
        }),
        Some(Rect::from_min_max(
            Point::new(59.0, 20.0),
            Point::new(121.0, 27.0)
        ))
    );
}

#[test]
fn canvas_selection_body_handle_rect_keeps_narrow_selection_movable() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_body_handle_rect(CanvasSelectionBodyHandleParts {
            bounds,
            start_fraction: 0.2,
            end_fraction: 0.205,
            height: 200.0,
            end_inset: 9.0,
            max_end_inset_fraction: 0.28,
            min_width_after_inset: 1.0,
        }),
        Some(Rect::from_min_max(
            Point::new(50.0, 20.0),
            Point::new(51.0, 120.0)
        ))
    );
}

#[test]
fn canvas_selection_trailing_control_rect_projects_bottom_square() {
    let bounds = Rect::from_min_max(Point::new(10.0, 20.0), Point::new(210.0, 120.0));

    assert_eq!(
        canvas_selection_trailing_control_rect(bounds, 0.2, 0.6, 16.0, 0.0),
        Some(Rect::from_min_max(
            Point::new(114.0, 104.0),
            Point::new(130.0, 120.0)
        ))
    );
    assert_eq!(
        canvas_selection_trailing_control_rect(bounds, 0.2, 0.6, 0.0, 0.0),
        None
    );
}
