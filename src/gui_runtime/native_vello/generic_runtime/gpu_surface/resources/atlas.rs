use super::super::*;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &surface.content else {
            return;
        };
        if self.textures.get(&surface.key).is_some_and(|texture| {
            texture.revision == surface.revision
                && texture.width == atlas.width
                && texture.height == atlas.height
        }) {
            return;
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_gpu_surface_texture"),
            size: wgpu::Extent3d {
                width: atlas.width as u32,
                height: atlas.height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            atlas.pixels.as_ref(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some((atlas.width * 4) as u32),
                rows_per_image: Some(atlas.height as u32),
            },
            wgpu::Extent3d {
                width: atlas.width as u32,
                height: atlas.height as u32,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.textures.insert(
            surface.key,
            GpuSurfaceTexture {
                revision: surface.revision,
                width: atlas.width,
                height: atlas.height,
                _texture: texture,
                view,
            },
        );
        stats.atlas_texture_uploads += 1;
    }
}
