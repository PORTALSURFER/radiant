use super::{target::PostGpuOverlayRenderTarget, vertex::OverlayVertex};
use crate::gui_runtime::native_vello::wgpu;

pub(super) struct PostGpuOverlayPipeline {
    format: wgpu::TextureFormat,
    device: usize,
    pipeline: wgpu::RenderPipeline,
}

impl PostGpuOverlayPipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
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
                        OverlayVertex::position_attribute(),
                        OverlayVertex::color_attribute(),
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
        Self {
            format,
            device: device_id(device),
            pipeline,
        }
    }

    pub(super) fn matches_target(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> bool {
        pipeline_matches_target(self.device, self.format, device_id(device), format)
    }

    pub(super) fn render(
        &self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
    ) {
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
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..vertex_count, 0..1);
    }
}

fn device_id(device: &wgpu::Device) -> usize {
    device as *const wgpu::Device as usize
}

fn pipeline_matches_target(
    cached_device: usize,
    cached_format: wgpu::TextureFormat,
    target_device: usize,
    target_format: wgpu::TextureFormat,
) -> bool {
    cached_device == target_device && cached_format == target_format
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

    #[test]
    fn cached_pipeline_matches_only_same_device_and_format() {
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        assert!(pipeline_matches_target(7, format, 7, format));
        assert!(!pipeline_matches_target(7, format, 8, format));
        assert!(!pipeline_matches_target(
            7,
            format,
            7,
            wgpu::TextureFormat::Rgba8UnormSrgb
        ));
    }
}
