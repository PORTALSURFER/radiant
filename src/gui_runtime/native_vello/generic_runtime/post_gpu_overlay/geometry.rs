use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    runtime::{PaintPrimitive, PaintStrokeRect},
};

use super::vertex::OverlayVertex;

pub(super) fn replayable_suffix(primitives: &[PaintPrimitive]) -> Option<&[PaintPrimitive]> {
    primitives
        .iter()
        .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
        .and_then(|index| primitives.get(index + 1..))
}

pub(super) fn gpu_surface_overlay_regions_into(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
) {
    regions.clear();
    regions.extend(primitives.iter().filter_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface)
            if surface.rect.width() > 0.0
                && surface.rect.height() > 0.0
                && surface.content.is_renderable() =>
        {
            Some(surface.rect)
        }
        _ => None,
    }));
}

#[cfg(test)]
pub(super) fn replayable_vertices_into(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    vertices: &mut Vec<OverlayVertex>,
) {
    vertices.clear();
    append_replayable_vertices(primitives, target_size, vertices);
}

pub(super) fn replayable_vertices_in_regions_into(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    regions: &[UiRect],
    vertices: &mut Vec<OverlayVertex>,
) {
    vertices.clear();
    append_replayable_vertices_in_regions(primitives, target_size, regions, vertices);
}

pub(super) fn append_replayable_vertices(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    vertices: &mut Vec<OverlayVertex>,
) {
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => {
                push_rect_vertices(vertices, target_size, fill.rect, fill.color);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                push_stroke_vertices(vertices, target_size, stroke);
            }
            _ => {}
        }
    }
}

pub(super) fn append_replayable_vertices_in_regions(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
    regions: &[UiRect],
    vertices: &mut Vec<OverlayVertex>,
) {
    if regions.is_empty() {
        return;
    }
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) if rect_intersects_any(fill.rect, regions) => {
                push_rect_vertices(vertices, target_size, fill.rect, fill.color);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                for edge in stroke_rect_edges(stroke.rect, stroke.width) {
                    if rect_intersects_any(edge, regions) {
                        push_rect_vertices(vertices, target_size, edge, stroke.color);
                    }
                }
            }
            _ => {}
        }
    }
}

fn push_stroke_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    stroke: &PaintStrokeRect,
) {
    for rect in stroke_rect_edges(stroke.rect, stroke.width) {
        push_rect_vertices(vertices, target_size, rect, stroke.color);
    }
}

fn push_rect_vertices(
    vertices: &mut Vec<OverlayVertex>,
    target_size: Vector2,
    rect: UiRect,
    color: Rgba8,
) {
    if rect.width() <= 0.0
        || rect.height() <= 0.0
        || color.a == 0
        || rect_is_outside_target(rect, target_size)
    {
        return;
    }
    let color = rgba_to_float(color);
    let left = clip_x(rect.min.x, target_size);
    let right = clip_x(rect.max.x, target_size);
    let top = clip_y(rect.min.y, target_size);
    let bottom = clip_y(rect.max.y, target_size);
    vertices.extend_from_slice(&[
        vertex(left, top, color),
        vertex(right, top, color),
        vertex(left, bottom, color),
        vertex(left, bottom, color),
        vertex(right, top, color),
        vertex(right, bottom, color),
    ]);
}

fn vertex(x: f32, y: f32, color: [f32; 4]) -> OverlayVertex {
    OverlayVertex::new([x, y], color)
}

fn clip_x(x: f32, target_size: Vector2) -> f32 {
    x / target_size.x.max(1.0) * 2.0 - 1.0
}

fn clip_y(y: f32, target_size: Vector2) -> f32 {
    1.0 - y / target_size.y.max(1.0) * 2.0
}

fn rect_is_outside_target(rect: UiRect, target_size: Vector2) -> bool {
    let target_width = target_size.x.max(0.0);
    let target_height = target_size.y.max(0.0);
    rect.max.x <= 0.0
        || rect.min.x >= target_width
        || rect.max.y <= 0.0
        || rect.min.y >= target_height
}

fn rect_intersects_any(rect: UiRect, regions: &[UiRect]) -> bool {
    regions.iter().any(|region| {
        rect.max.x > region.min.x
            && rect.min.x < region.max.x
            && rect.max.y > region.min.y
            && rect.min.y < region.max.y
    })
}

fn stroke_rect_edges(rect: UiRect, width: f32) -> [UiRect; 4] {
    let width = width.max(1.0);
    [
        UiRect::from_min_size(rect.min, Vector2::new(rect.width(), width)),
        UiRect::from_min_size(
            Point::new(rect.min.x, rect.max.y - width),
            Vector2::new(rect.width(), width),
        ),
        UiRect::from_min_size(rect.min, Vector2::new(width, rect.height())),
        UiRect::from_min_size(
            Point::new(rect.max.x - width, rect.min.y),
            Vector2::new(width, rect.height()),
        ),
    ]
}

fn rgba_to_float(color: Rgba8) -> [f32; 4] {
    [
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a as f32 / 255.0,
    ]
}

#[cfg(test)]
#[path = "geometry/tests.rs"]
mod tests;
