use crate::{
    gui::types::{Point, Rect as UiRect, Rgba8, Vector2},
    gui_runtime::native_vello::wgpu,
    runtime::{PaintPrimitive, PaintStrokeRect},
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(super) struct OverlayVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl OverlayVertex {
    pub(super) fn position_attribute() -> wgpu::VertexAttribute {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }
    }

    pub(super) fn color_attribute() -> wgpu::VertexAttribute {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            shader_location: 1,
        }
    }
}

pub(super) fn replayable_suffix(primitives: &[PaintPrimitive]) -> Option<&[PaintPrimitive]> {
    primitives
        .iter()
        .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
        .and_then(|index| primitives.get(index + 1..))
}

pub(super) fn replayable_vertices(
    primitives: &[PaintPrimitive],
    target_size: Vector2,
) -> Vec<OverlayVertex> {
    let mut vertices = Vec::new();
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => {
                push_rect_vertices(&mut vertices, target_size, fill.rect, fill.color);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                push_stroke_vertices(&mut vertices, target_size, stroke);
            }
            _ => {}
        }
    }
    vertices
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
    if rect.width() <= 0.0 || rect.height() <= 0.0 || color.a == 0 {
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
    OverlayVertex {
        position: [x, y],
        color,
    }
}

fn clip_x(x: f32, target_size: Vector2) -> f32 {
    x / target_size.x.max(1.0) * 2.0 - 1.0
}

fn clip_y(y: f32, target_size: Vector2) -> f32 {
    1.0 - y / target_size.y.max(1.0) * 2.0
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

pub(super) fn overlay_vertex_bytes(vertices: &[OverlayVertex]) -> &[u8] {
    let size = std::mem::size_of_val(vertices);
    let ptr = vertices.as_ptr() as *const u8;
    // SAFETY: `OverlayVertex` is a repr(C) POD-like value containing only f32
    // arrays. The slice is used only while wgpu copies the bytes into a buffer.
    unsafe { std::slice::from_raw_parts(ptr, size) }
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

        let vertices = replayable_vertices(&primitives, Vector2::new(100.0, 50.0));

        assert_eq!(vertices.len(), 30);
        assert_eq!(vertices[0].position, [-1.0, 1.0]);
        assert_eq!(vertices[5].position, [-0.98, 0.96]);
    }

    #[test]
    fn overlay_vertices_keep_vertex_buffer_stride_stable() {
        assert_eq!(std::mem::size_of::<OverlayVertex>(), 24);
        assert_eq!(
            OverlayVertex::color_attribute().offset,
            std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
        );
    }

    #[test]
    fn overlay_vertex_bytes_cover_all_vertices() {
        let vertices = replayable_vertices(&[fill(1)], Vector2::new(100.0, 50.0));

        assert_eq!(
            overlay_vertex_bytes(&vertices).len(),
            vertices.len() * std::mem::size_of::<OverlayVertex>()
        );
    }

    fn fill(widget_id: u64) -> PaintPrimitive {
        PaintPrimitive::FillRect(PaintFillRect {
            widget_id,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            color: white(),
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
