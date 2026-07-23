use super::super::*;
use crate::runtime::{PaintFillPolygon, PaintStrokePolygon, PaintSvg, PaintSvgDocument};
use std::sync::Arc;

fn polygon_primitives() -> [PaintPrimitive; 2] {
    let points = Arc::from([
        Point::new(10.0, 10.0),
        Point::new(30.0, 10.0),
        Point::new(28.0, 30.0),
        Point::new(10.0, 30.0),
    ]);
    [
        PaintPrimitive::FillPolygon(PaintFillPolygon {
            widget_id: 41,
            points: Arc::clone(&points),
            color: Rgba8::new(40, 80, 160, 96),
        }),
        PaintPrimitive::StrokePolygon(PaintStrokePolygon {
            widget_id: 41,
            points,
            color: Rgba8::new(100, 160, 255, 255),
            width: 2.0,
        }),
    ]
}

fn svg_primitive() -> PaintPrimitive {
    let document = PaintSvgDocument::from_svg(
        r##"<svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
  <path fill="#6699ff" d="M2 2h12v12H2z"/>
  <path fill="none" stroke="#ffffff" stroke-width="1" d="M4 8h8"/>
</svg>"##,
    )
    .expect("valid retained icon");
    PaintPrimitive::Svg(PaintSvg {
        widget_id: 42,
        document,
        rect: UiRect::from_min_size(Point::new(8.0, 8.0), Vector2::new(24.0, 24.0)),
    })
}

#[test]
fn polygon_chrome_and_svg_icons_are_replayable() {
    let polygons = polygon_primitives();
    let svg = svg_primitive();

    assert!(polygons.iter().all(primitive_is_replayable));
    assert!(primitive_is_replayable(&svg));
}

#[test]
fn transient_overlay_replay_keeps_polygon_chrome_and_svg_icon_geometry() {
    let mut primitives = polygon_primitives().to_vec();
    primitives.push(svg_primitive());
    let mut vertices = Vec::new();

    replayable_vertices_into(&primitives, Vector2::new(100.0, 60.0), &mut vertices);

    assert!(
        vertices.len() > 18,
        "fill, border, and retained SVG paths should all contribute geometry"
    );
    assert!(vertices.iter().all(|vertex| {
        vertex
            .position
            .iter()
            .all(|component| component.is_finite())
    }));
}

#[test]
fn clipped_suffix_replay_keeps_opaque_svg_icons_inside_gpu_regions() {
    let svg = svg_primitive();
    let regions = [UiRect::from_min_size(
        Point::new(16.0, 12.0),
        Vector2::new(8.0, 12.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        std::slice::from_ref(&svg),
        Vector2::new(100.0, 60.0),
        &regions,
        &mut vertices,
    );

    assert!(
        !vertices.is_empty(),
        "opaque SVG icon paths still need replay because their transparent bounds cannot occlude the GPU surface"
    );
    assert!(vertices.iter().all(|vertex| {
        (-0.6801..=-0.5199).contains(&vertex.position[0])
            && (0.1999..=0.6001).contains(&vertex.position[1])
    }));
}

#[test]
fn clipped_suffix_replay_keeps_polygon_fill_and_border() {
    let polygons = polygon_primitives();
    let regions = [UiRect::from_min_size(
        Point::new(12.0, 12.0),
        Vector2::new(10.0, 10.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        &polygons,
        Vector2::new(100.0, 60.0),
        &regions,
        &mut vertices,
    );

    assert!(!vertices.is_empty());
    assert!(vertices.iter().all(|vertex| {
        (-0.7601..=-0.5599).contains(&vertex.position[0])
            && (0.2665..=0.6001).contains(&vertex.position[1])
    }));
}
