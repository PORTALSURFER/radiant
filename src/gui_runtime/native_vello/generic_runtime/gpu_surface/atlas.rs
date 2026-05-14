use super::*;
use crate::runtime::PaintGpuSurface;

pub(super) struct TextureViewRenderRequest<'a> {
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) texture_identity: GpuSurfaceTextureIdentity,
    pub(super) texture_view: &'a wgpu::TextureView,
    pub(super) source: [f32; 4],
    pub(super) occlusion_regions: &'a [UiRect],
}

impl GpuSurfaceRenderer {
    pub(super) fn render_atlas(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        source_rect: UiRect,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.ensure_texture(target.device, target.queue, surface, stats);
        self.ensure_pipeline(target.device, target.format);
        let Some(texture) = self.textures.get(&surface.key) else {
            return;
        };
        let texture_identity = GpuSurfaceTextureIdentity::RgbaAtlas {
            revision: texture.revision,
            width: texture.width,
            height: texture.height,
        };
        let texture_view = texture.view.clone();
        self.render_texture_view(
            target,
            TextureViewRenderRequest {
                surface,
                texture_identity,
                texture_view: &texture_view,
                source: [
                    source_rect.min.x,
                    source_rect.min.y,
                    source_rect.width(),
                    source_rect.height(),
                ],
                occlusion_regions,
            },
            stats,
        );
    }

    pub(super) fn render_texture_view(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: TextureViewRenderRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(pipeline) = self.pipeline.as_ref() else {
            return;
        };
        let surface = request.surface;
        let (overlay_ratios, overlay_widths, overlay_colors) = vertical_overlays(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
            source: request.source,
            target_size: [target.size.x.max(1.0), target.size.y.max(1.0)],
            _padding: [0.0; 2],
            overlay_ratios,
            overlay_widths,
            overlay_colors,
        };
        let cache_key = GpuSurfaceCompositeBindingKey {
            pipeline_generation: self.pipeline_generation,
            texture: request.texture_identity,
        };
        let rebuild_binding = self
            .composite_bindings
            .get(&surface.key)
            .is_none_or(|binding| binding.cache_key != cache_key);
        if rebuild_binding {
            stats.composite_binding_rebuilds += 1;
            let uniform_buffer = target.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("radiant_gpu_surface_uniforms"),
                size: std::mem::size_of::<GpuSurfaceUniforms>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
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
                        resource: wgpu::BindingResource::TextureView(request.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            });
            self.composite_bindings.insert(
                surface.key,
                GpuSurfaceCompositeBinding {
                    cache_key,
                    uniform_buffer,
                    bind_group,
                },
            );
        } else {
            stats.composite_binding_cache_hits += 1;
        }
        let Some(binding) = self.composite_bindings.get(&surface.key) else {
            return;
        };
        target
            .queue
            .write_buffer(&binding.uniform_buffer, 0, uniforms_as_bytes(&uniforms));
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &binding.bind_group, &[]);
        for region in visible_surface_regions(surface.rect, request.occlusion_regions) {
            set_surface_scissor(&mut pass, region);
            pass.draw(0..6, 0..1);
        }
        stats.composite_encode_elapsed += started.elapsed();
    }
}
