use super::*;
use crate::runtime::PaintGpuSurface;
use wgpu::util::DeviceExt;

impl GpuSurfaceRenderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        source_rect: UiRect,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.ensure_texture(device, queue, surface);
        self.ensure_pipeline(device, target_format);
        let Some(texture) = self.textures.get(&surface.key) else {
            return;
        };
        self.render_texture_view(
            device,
            encoder,
            target_view,
            target_format,
            target_size,
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

    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_texture_view(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        texture_view: &wgpu::TextureView,
        source: [f32; 4],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let _ = target_format;
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let cursor = vertical_cursor(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            source,
            target_size: [target_size.x.max(1.0), target_size.y.max(1.0)],
            cursor_ratio: cursor.map(|cursor| cursor.0).unwrap_or(-1.0),
            cursor_width: cursor.map(|cursor| cursor.2).unwrap_or(1.0),
            cursor_color: cursor
                .map(|cursor| rgba_to_float(cursor.1))
                .unwrap_or([1.0, 1.0, 1.0, 0.92]),
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("radiant_gpu_surface_uniforms"),
            contents: uniforms_as_bytes(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut pass = gpu_surface_render_pass(encoder, target_view);
        set_surface_scissor(&mut pass, surface.rect);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
        stats.composite_encode_elapsed += started.elapsed();
    }
}
