//! Immediate shape replay for paint ordered above native GPU surfaces.

use super::*;
use wgpu::util::DeviceExt;

#[derive(Default)]
pub(super) struct PostGpuOverlayRenderer {
    pipeline: Option<PostGpuOverlayPipeline>,
}

pub(super) struct PostGpuOverlayRenderTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) format: wgpu::TextureFormat,
    pub(super) size: Vector2,
}

struct PostGpuOverlayPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct OverlayVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl PostGpuOverlayRenderer {
    pub(super) fn render(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[PaintPrimitive],
    ) {
        let Some(suffix) = primitives
            .iter()
            .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
            .and_then(|index| primitives.get(index + 1..))
        else {
            return;
        };
        let vertices = replayable_vertices(suffix, target.size);
        if vertices.is_empty() {
            return;
        }
        self.ensure_pipeline(target.device, target.format);
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let vertex_buffer = target
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("radiant_post_gpu_overlay_vertices"),
                contents: overlay_vertex_bytes(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let mut pass = target
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("radiant_post_gpu_overlay_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..vertices.len() as u32, 0..1);
    }

    fn ensure_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self
            .pipeline
            .as_ref()
            .is_some_and(|pipeline| pipeline.format == format)
        {
            return;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("radiant_post_gpu_overlay_shader"),
            source: wgpu::ShaderSource::Wgsl(POST_GPU_OVERLAY_SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radiant_post_gpu_overlay_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_post_gpu_overlay_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<OverlayVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        self.pipeline = Some(PostGpuOverlayPipeline { format, pipeline });
    }
}

fn replayable_vertices(primitives: &[PaintPrimitive], target_size: Vector2) -> Vec<OverlayVertex> {
    let mut vertices = Vec::new();
    for primitive in primitives {
        match primitive {
            PaintPrimitive::FillRect(fill) => {
                push_rect_vertices(&mut vertices, target_size, fill.rect, fill.color);
            }
            PaintPrimitive::StrokeRect(stroke) => {
                for rect in stroke_rect_edges(stroke.rect, stroke.width) {
                    push_rect_vertices(&mut vertices, target_size, rect, stroke.color);
                }
            }
            _ => {}
        }
    }
    vertices
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

fn overlay_vertex_bytes(vertices: &[OverlayVertex]) -> &[u8] {
    let size = std::mem::size_of_val(vertices);
    let ptr = vertices.as_ptr() as *const u8;
    // SAFETY: `OverlayVertex` is a repr(C) POD-like value containing only f32
    // arrays. The slice is used only for the duration
    // of this call while wgpu copies the bytes into an owned buffer.
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

const POST_GPU_OVERLAY_SHADER: &str = r#"
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{PaintFillRect, PaintStrokeRect};

    #[test]
    fn replayable_suffix_starts_after_last_gpu_surface() {
        let primitives = [fill(1), gpu(2), fill(3), gpu(4), stroke(5), fill(6)];
        let suffix = primitives
            .iter()
            .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
            .and_then(|index| primitives.get(index + 1..))
            .expect("suffix");

        assert_eq!(suffix.len(), 2);
        assert!(matches!(suffix[0], PaintPrimitive::StrokeRect(_)));
        assert!(matches!(suffix[1], PaintPrimitive::FillRect(_)));
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
        PaintPrimitive::GpuSurface(crate::runtime::PaintGpuSurface {
            widget_id,
            key: widget_id,
            revision: 0,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            content: crate::runtime::GpuSurfaceContent::RgbaAtlas {
                atlas: Arc::new(
                    crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                        .expect("valid one-pixel image"),
                ),
                source_rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            },
            capabilities: crate::runtime::GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })
    }
}
