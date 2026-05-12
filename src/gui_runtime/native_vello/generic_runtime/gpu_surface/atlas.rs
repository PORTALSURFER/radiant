use super::*;
use crate::runtime::PaintGpuSurface;
use wgpu::util::DeviceExt;

impl GpuSurfaceRenderer {
    pub(super) fn render_atlas(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        source_rect: UiRect,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.ensure_texture(target.device, target.queue, surface, stats);
        self.ensure_pipeline(target.device, target.format);
        let Some(texture) = self.textures.get(&surface.key) else {
            return;
        };
        self.render_texture_view(
            target,
            surface,
            &texture.view,
            [
                source_rect.min.x,
                source_rect.min.y,
                source_rect.width(),
                source_rect.height(),
            ],
            stats,
        );
    }

    pub(super) fn render_texture_view(
        &self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        texture_view: &wgpu::TextureView,
        source: [f32; 4],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let (overlay_ratios, overlay_widths, overlay_colors) = vertical_overlays(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            source,
            target_size: [target.size.x.max(1.0), target.size.y.max(1.0)],
            overlay_ratios,
            overlay_widths,
            overlay_colors,
        };
        let uniform_buffer = target
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("radiant_gpu_surface_uniforms"),
                contents: uniforms_as_bytes(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind_group = target.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("radiant_gpu_surface_bind_group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
        set_surface_scissor(&mut pass, surface.rect);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
        stats.composite_encode_elapsed += started.elapsed();
    }
}
