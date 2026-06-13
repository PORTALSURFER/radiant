use super::fixtures::*;

#[test]
fn horizontal_resize_handles_prioritize_edges_over_body() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));
    let handles = horizontal_resize_handles(rect, 12.0, 99).expect("handles");

    assert_eq!(handles[0].role, DragHandleRole::Body);
    assert_eq!(handles[0].capture_token, 99);
    assert_eq!(
        handles[1].rect,
        Rect::from_min_max(Point::new(20.0, 30.0), Point::new(32.0, 70.0))
    );
    assert_eq!(
        handles[2].rect,
        Rect::from_min_max(Point::new(108.0, 30.0), Point::new(120.0, 70.0))
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(25.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::Start)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(60.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::Body)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(116.0, 50.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
}

#[test]
fn horizontal_resize_edges_clamp_to_half_width() {
    let rect = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(10.0, 20.0));
    let handles = horizontal_resize_edge_handles(rect, 12.0, 7).expect("handles");

    assert_eq!(
        handles[0].rect,
        Rect::from_min_max(Point::new(0.0, 0.0), Point::new(5.0, 20.0))
    );
    assert_eq!(
        handles[1].rect,
        Rect::from_min_max(Point::new(5.0, 0.0), Point::new(10.0, 20.0))
    );
    assert_eq!(horizontal_resize_edge_handles(rect, 0.0, 7), None);
}

#[test]
fn horizontal_resize_edge_visual_rect_supports_insets() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));

    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::Start, 5.0, 2.0, 4.0),
        Some(Rect::from_min_max(
            Point::new(22.0, 34.0),
            Point::new(27.0, 66.0)
        ))
    );
    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::End, 5.0, 2.0, 4.0),
        Some(Rect::from_min_max(
            Point::new(113.0, 34.0),
            Point::new(118.0, 66.0)
        ))
    );
    assert_eq!(
        horizontal_resize_edge_visual_rect(rect, DragHandleRole::Body, 5.0, 2.0, 4.0),
        None
    );
}

#[test]
fn horizontal_resize_edge_bracket_rects_project_edge_and_ticks() {
    let rect = Rect::from_min_max(Point::new(20.0, 30.0), Point::new(120.0, 70.0));

    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::Start, 2.0, 7.0),
        Some([
            Rect::from_min_max(Point::new(20.0, 30.0), Point::new(22.0, 70.0)),
            Rect::from_min_max(Point::new(20.0, 30.0), Point::new(27.0, 32.0)),
            Rect::from_min_max(Point::new(20.0, 68.0), Point::new(27.0, 70.0)),
        ])
    );
    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::End, 2.0, 7.0),
        Some([
            Rect::from_min_max(Point::new(118.0, 30.0), Point::new(120.0, 70.0)),
            Rect::from_min_max(Point::new(113.0, 30.0), Point::new(120.0, 32.0)),
            Rect::from_min_max(Point::new(113.0, 68.0), Point::new(120.0, 70.0)),
        ])
    );
    assert_eq!(
        horizontal_resize_edge_bracket_rects(rect, DragHandleRole::Body, 2.0, 7.0),
        None
    );
}
