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

pub(super) fn gpu_surface_overlay_regions(primitives: &[PaintPrimitive]) -> Vec<UiRect> {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(surface)
                if surface.rect.width() > 0.0
                    && surface.rect.height() > 0.0
                    && surface.content.is_renderable() =>
            {
                Some(surface.rect)
            }
            _ => None,
        })
        .collect()
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
mod tests {
    use super::*;
    use crate::runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintGpuSurface,
    };
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
                white(),
            ),
            rect(
                UiRect::from_min_size(Point::new(0.0, 30.0), Vector2::new(10.0, 10.0)),
                white(),
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
}
