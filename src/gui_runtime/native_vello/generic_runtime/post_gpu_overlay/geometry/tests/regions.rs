use super::{super::*, fixtures::*};

#[test]
fn replayable_vertices_in_regions_reject_invalid_region_geometry() {
    let primitives = [rect(
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 30.0)),
        translucent_white(),
    )];
    let regions = [UiRect::from_min_size(
        Point::new(f32::NAN, 0.0),
        Vector2::new(10.0, 10.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert!(vertices.is_empty());
}

#[test]
fn replayable_vertices_in_regions_skip_unrelated_later_rectangles() {
    let primitives = [
        rect(
            UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            translucent_white(),
        ),
        rect(
            UiRect::from_min_size(Point::new(0.0, 30.0), Vector2::new(10.0, 10.0)),
            translucent_white(),
        ),
    ];
    let regions = [UiRect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(12.0, 12.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert_eq!(vertices.len(), 6);
}

#[test]
fn replayable_vertices_in_regions_clip_translucent_fills_to_gpu_regions() {
    let primitives = [rect(
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 30.0)),
        translucent_white(),
    )];
    let regions = [UiRect::from_min_size(
        Point::new(10.0, 10.0),
        Vector2::new(10.0, 10.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert_eq!(vertices.len(), 6);
    assert_eq!(vertices[0].position, [-0.8, 0.6]);
    assert!((vertices[5].position[0] - -0.6).abs() < 0.0001);
    assert!((vertices[5].position[1] - 0.2).abs() < 0.0001);
}

#[test]
fn replayable_vertices_in_regions_skip_opaque_fills_revealed_by_gpu_clipping() {
    let primitives = [rect(
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(30.0, 30.0)),
        white(),
    )];
    let regions = [UiRect::from_min_size(
        Point::new(10.0, 10.0),
        Vector2::new(10.0, 10.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert!(vertices.is_empty());
}

#[test]
fn replayable_vertices_in_regions_keep_overlapping_stroke_edges() {
    let primitives = [PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: 7,
        rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
        color: white(),
        width: 2.0,
    })];
    let regions = [UiRect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(40.0, 40.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert_eq!(vertices.len(), 24);
}

#[test]
fn replayable_vertices_in_regions_clip_stroke_edges_to_gpu_regions() {
    let primitives = [PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: 7,
        rect: UiRect::from_min_size(Point::new(0.0, 10.0), Vector2::new(40.0, 20.0)),
        color: white(),
        width: 2.0,
    })];
    let regions = [UiRect::from_min_size(
        Point::new(10.0, 0.0),
        Vector2::new(10.0, 50.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert_eq!(vertices.len(), 12);
    assert!((vertices[0].position[0] - -0.8).abs() < 0.0001);
    assert!((vertices[1].position[0] - -0.6).abs() < 0.0001);
}

#[test]
fn replayable_vertices_in_regions_clip_text_without_changing_paint_run() {
    let primitive = text("kick.wav");
    let PaintPrimitive::Text(original_text) = &primitive else {
        panic!("fixture should produce text primitive");
    };
    let original_rect = original_text.rect;
    let primitives = [primitive];
    let regions = [UiRect::from_min_size(
        Point::new(4.0, 4.0),
        Vector2::new(12.0, 18.0),
    )];
    let mut full_vertices = Vec::new();
    let mut clipped_vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(160.0, 60.0), &mut full_vertices);
    replayable_vertices_in_regions_into(
        &primitives,
        Vector2::new(160.0, 60.0),
        &regions,
        &mut clipped_vertices,
    );

    assert!(!clipped_vertices.is_empty());
    assert!(clipped_vertices.len() < full_vertices.len());
    let PaintPrimitive::Text(text) = &primitives[0] else {
        panic!("fixture should remain a text primitive");
    };
    assert_eq!(text.rect, original_rect);
}
