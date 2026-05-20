use super::{super::*, fixtures::*};

#[test]
fn stroke_rect_edges_emit_four_positive_rects() {
    let rect = UiRect::from_min_size(Point::new(10.0, 20.0), Vector2::new(100.0, 40.0));

    let edges = stroke_rect_edges(rect, 2.0);

    assert_eq!(edges.len(), 4);
    assert!(edges.iter().all(|edge| edge.width() > 0.0));
    assert!(edges.iter().all(|edge| edge.height() > 0.0));
}

#[test]
fn replayable_vertices_batch_fill_and_stroke_rectangles() {
    let primitives = [fill(1), stroke(2)];
    let mut vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(vertices.len(), 30);
    assert_eq!(vertices[0].position, [-1.0, 1.0]);
    assert_eq!(vertices[5].position, [-0.98, 0.96]);
}

#[test]
fn replayable_vertices_reuses_existing_storage() {
    let primitives = [fill(1), stroke(2)];
    let mut vertices = Vec::with_capacity(64);

    replayable_vertices_into(&primitives, Vector2::new(100.0, 50.0), &mut vertices);
    let capacity = vertices.capacity();
    replayable_vertices_into(&[fill(3)], Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(capacity, 64);
    assert_eq!(vertices.capacity(), capacity);
    assert_eq!(vertices.len(), 6);
}

#[test]
fn replayable_vertices_skip_fully_offscreen_rectangles() {
    let primitives = [rect(
        UiRect::from_min_size(Point::new(120.0, 0.0), Vector2::new(10.0, 10.0)),
        white(),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(100.0, 50.0), &mut vertices);

    assert!(vertices.is_empty());
}

#[test]
fn replayable_vertices_keep_partially_visible_rectangles() {
    let primitives = [rect(
        UiRect::from_min_size(Point::new(95.0, 0.0), Vector2::new(10.0, 10.0)),
        white(),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(vertices.len(), 6);
}

#[test]
fn replayable_vertices_reject_invalid_target_and_rect_geometry() {
    let invalid_rect = rect(
        UiRect::from_min_size(Point::new(f32::NAN, 0.0), Vector2::new(10.0, 10.0)),
        white(),
    );
    let valid_rect = rect(
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
        white(),
    );
    let mut vertices = Vec::new();

    replayable_vertices_into(
        std::slice::from_ref(&valid_rect),
        Vector2::new(f32::INFINITY, 50.0),
        &mut vertices,
    );
    assert!(vertices.is_empty());

    replayable_vertices_into(&[invalid_rect], Vector2::new(100.0, 50.0), &mut vertices);
    assert!(vertices.is_empty());
}

#[test]
fn append_replayable_vertices_preserves_existing_vertices() {
    let mut vertices = Vec::new();

    replayable_vertices_into(&[fill(1)], Vector2::new(100.0, 50.0), &mut vertices);
    append_replayable_vertices(&[fill(2)], Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(vertices.len(), 12);
}

#[test]
fn replayable_vertices_sanitize_invalid_stroke_widths() {
    let primitives = [PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: 7,
        rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
        color: white(),
        width: f32::NAN,
    })];
    let mut vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(vertices.len(), 24);
    assert!(
        vertices
            .iter()
            .all(|vertex| vertex.position.iter().all(|value| value.is_finite()))
    );
}
