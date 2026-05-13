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
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct RectUniforms {
    dest: [f32; 4],
    target_size: [f32; 2],
    _padding: [f32; 2],
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
        if !suffix.iter().any(is_replayable_shape) {
            return;
        }
        self.ensure_pipeline(target.device, target.format);
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let device = target.device;
        let size = target.size;
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
        for primitive in suffix {
            match primitive {
                PaintPrimitive::FillRect(fill) => {
                    draw_rect(device, size, &mut pass, pipeline, fill.rect, fill.color);
                }
                PaintPrimitive::StrokeRect(stroke) => {
                    for rect in stroke_rect_edges(stroke.rect, stroke.width) {
                        draw_rect(device, size, &mut pass, pipeline, rect, stroke.color);
                    }
                }
                _ => {}
            }
        }
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
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("radiant_post_gpu_overlay_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("radiant_post_gpu_overlay_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("radiant_post_gpu_overlay_pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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
        self.pipeline = Some(PostGpuOverlayPipeline {
            format,
            bind_group_layout,
            pipeline,
        });
    }
}

fn is_replayable_shape(primitive: &PaintPrimitive) -> bool {
    matches!(
        primitive,
        PaintPrimitive::FillRect(_) | PaintPrimitive::StrokeRect(_)
    )
}

fn draw_rect(
    device: &wgpu::Device,
    target_size: Vector2,
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &PostGpuOverlayPipeline,
    rect: UiRect,
    color: Rgba8,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 || color.a == 0 {
        return;
    }
    let uniforms = RectUniforms {
        dest: [rect.min.x, rect.min.y, rect.width(), rect.height()],
        target_size: [target_size.x.max(1.0), target_size.y.max(1.0)],
        _padding: [0.0; 2],
        color: rgba_to_float(color),
    };
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("radiant_post_gpu_overlay_uniforms"),
        contents: uniform_bytes(&uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("radiant_post_gpu_overlay_bind_group"),
        layout: &pipeline.bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..6, 0..1);
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

fn uniform_bytes<T>(uniforms: &T) -> &[u8] {
    let size = std::mem::size_of::<T>();
    let ptr = uniforms as *const T as *const u8;
    // SAFETY: `uniforms` is a plain repr(C) value used only for the duration
    // of this call while wgpu copies the bytes into an owned buffer.
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

const POST_GPU_OVERLAY_SHADER: &str = r#"
struct Params {
    dest: vec4<f32>,
    target_size: vec2<f32>,
    color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> params: Params;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let local = corners[vertex_index];
    let pixel = params.dest.xy + local * params.dest.zw;
    let clip = vec2<f32>(
        pixel.x / params.target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / params.target_size.y * 2.0,
    );
    var out: VertexOut;
    out.position = vec4<f32>(clip, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(_in: VertexOut) -> @location(0) vec4<f32> {
    return params.color;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{PaintFillRect, PaintStrokeRect};

    #[test]
    fn replayable_suffix_starts_after_last_gpu_surface() {
        let primitives = vec![fill(1), gpu(2), fill(3), gpu(4), stroke(5), fill(6)];
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
    fn rect_uniforms_keep_wgsl_vec4_alignment() {
        assert_eq!(std::mem::size_of::<RectUniforms>(), 48);
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
