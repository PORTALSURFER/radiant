use super::super::*;
use crate::runtime::{
    PaintBrush, PaintFillPath, PaintLinearGradient, PaintPath, PaintPathCommand, PaintPrimitive,
};

fn gradient_path() -> PaintPrimitive {
    let bounds = UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(60.0, 30.0));
    let path = PaintPath::from([
        PaintPathCommand::MoveTo(bounds.min),
        PaintPathCommand::LineTo(Point::new(bounds.max.x, bounds.min.y)),
        PaintPathCommand::LineTo(bounds.max),
        PaintPathCommand::LineTo(Point::new(bounds.min.x, bounds.max.y)),
        PaintPathCommand::Close,
    ]);
    PaintPrimitive::FillPath(PaintFillPath::new(
        91,
        path,
        PaintBrush::linear_gradient(PaintLinearGradient::vertical(
            bounds,
            Rgba8::new(255, 80, 40, 160),
            Rgba8::new(255, 80, 40, 8),
        )),
    ))
}

fn opaque_path() -> PaintPrimitive {
    let path = PaintPath::from([
        PaintPathCommand::MoveTo(Point::new(10.0, 10.0)),
        PaintPathCommand::LineTo(Point::new(70.0, 10.0)),
        PaintPathCommand::LineTo(Point::new(70.0, 40.0)),
        PaintPathCommand::LineTo(Point::new(10.0, 40.0)),
        PaintPathCommand::Close,
    ]);
    PaintPrimitive::FillPath(PaintFillPath::new(
        92,
        path,
        PaintBrush::solid(Rgba8::new(20, 30, 40, 255)),
    ))
}

#[test]
fn replayable_gradient_fill_path_preserves_alpha_ramp() {
    let primitive = gradient_path();
    let mut vertices = Vec::new();

    assert!(primitive_is_replayable(&primitive));
    replayable_vertices_into(
        std::slice::from_ref(&primitive),
        Vector2::new(100.0, 50.0),
        &mut vertices,
    );

    assert_eq!(vertices.len(), 6);
    let min_alpha = vertices
        .iter()
        .map(|vertex| vertex.color[3])
        .fold(f32::INFINITY, f32::min);
    let max_alpha = vertices
        .iter()
        .map(|vertex| vertex.color[3])
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(min_alpha < 0.05);
    assert!(max_alpha > 0.6);
    assert!(vertices.iter().all(|vertex| {
        vertex
            .position
            .iter()
            .all(|component| component.is_finite())
    }));
}

#[test]
fn replayable_gradient_fill_path_clips_to_gpu_regions() {
    let primitive = gradient_path();
    let regions = [UiRect::from_min_size(
        Point::new(20.0, 20.0),
        Vector2::new(20.0, 10.0),
    )];
    let mut vertices = Vec::new();

    replayable_vertices_in_regions_into(
        std::slice::from_ref(&primitive),
        Vector2::new(100.0, 50.0),
        &regions,
        &mut vertices,
    );

    assert!(!vertices.is_empty());
    assert!(vertices.iter().all(|vertex| {
        (-0.6001..=-0.1999).contains(&vertex.position[0])
            && (-0.2001..=0.2001).contains(&vertex.position[1])
    }));
}

#[test]
fn opaque_fill_path_skips_gpu_region_replay_but_remains_full_overlay_replay() {
    let primitive = opaque_path();
    let regions = [UiRect::from_min_size(
        Point::new(20.0, 20.0),
        Vector2::new(20.0, 10.0),
    )];
    let mut full_vertices = Vec::new();
    let mut region_vertices = Vec::new();

    replayable_vertices_into(
        std::slice::from_ref(&primitive),
        Vector2::new(100.0, 50.0),
        &mut full_vertices,
    );
    replayable_vertices_in_regions_into(
        std::slice::from_ref(&primitive),
        Vector2::new(100.0, 50.0),
        &regions,
        &mut region_vertices,
    );

    assert!(!full_vertices.is_empty());
    assert!(region_vertices.is_empty());
}
