use super::*;
use crate::runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintGpuSurface};
use std::sync::Arc;

#[test]
fn replayable_suffix_starts_after_last_gpu_surface() {
    let primitives = [fill(1), gpu(2), fill(3), gpu(4), stroke(5), fill(6)];
    let suffix = replayable_suffix(&primitives).expect("suffix");

    assert_eq!(suffix.len(), 2);
    assert!(matches!(suffix[0], PaintPrimitive::StrokeRect(_)));
    assert!(matches!(suffix[1], PaintPrimitive::FillRect(_)));
}

#[test]
fn replayable_suffix_is_absent_when_no_gpu_surface_exists() {
    let primitives = [fill(1), stroke(2)];

    assert!(replayable_suffix(&primitives).is_none());
}

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
fn append_replayable_vertices_preserves_existing_vertices() {
    let mut vertices = Vec::new();

    replayable_vertices_into(&[fill(1)], Vector2::new(100.0, 50.0), &mut vertices);
    append_replayable_vertices(&[fill(2)], Vector2::new(100.0, 50.0), &mut vertices);

    assert_eq!(vertices.len(), 12);
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

fn fill(widget_id: u64) -> PaintPrimitive {
    fill_rect(
        widget_id,
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        white(),
    )
}

fn rect(rect: UiRect, color: Rgba8) -> PaintPrimitive {
    fill_rect(1, rect, color)
}

fn fill_rect(widget_id: u64, rect: UiRect, color: Rgba8) -> PaintPrimitive {
    PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    })
}

fn stroke(widget_id: u64) -> PaintPrimitive {
    PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        color: white(),
        width: 1.0,
    })
}

fn white() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    }
}

fn translucent_white() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 160,
    }
}

fn gpu(widget_id: u64) -> PaintPrimitive {
    PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id,
        key: widget_id,
        revision: 0,
        rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        content: GpuSurfaceContent::RgbaAtlas {
            atlas: Arc::new(
                crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                    .expect("valid one-pixel image"),
            ),
            source_rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    })
}
