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
        if self
            .resources
            .textures
            .get(&surface.key)
            .is_some_and(|texture| {
                texture.matches_atlas(device, surface.revision, atlas.width, atlas.height)
            })
        {
            stats.atlas_texture_cache_hits += 1;
            return;
        }
        let Some(extent) = GpuAtlasTextureExtent::new(atlas.width, atlas.height) else {
            return;
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_gpu_surface_texture"),
            size: wgpu::Extent3d {
                width: extent.width,
                height: extent.height,
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
                bytes_per_row: Some(extent.bytes_per_row),
                rows_per_image: Some(extent.height),
            },
            wgpu::Extent3d {
                width: extent.width,
                height: extent.height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.resources.textures.insert(
            surface.key,
            GpuSurfaceTexture {
                device: wgpu_device_id(device),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuAtlasTextureExtent {
    width: u32,
    height: u32,
    bytes_per_row: u32,
}

impl GpuAtlasTextureExtent {
    fn new(width: usize, height: usize) -> Option<Self> {
        let width = u32::try_from(width).ok()?;
        let height = u32::try_from(height).ok()?;
        if width == 0 || height == 0 {
            return None;
        }
        let bytes_per_row = width.checked_mul(4)?;
        Some(Self {
            width,
            height,
            bytes_per_row,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::GpuAtlasTextureExtent;

    #[test]
    fn gpu_atlas_texture_extent_rejects_empty_or_oversized_dimensions() {
        assert_eq!(GpuAtlasTextureExtent::new(0, 1), None);
        assert_eq!(GpuAtlasTextureExtent::new(1, 0), None);
        assert_eq!(GpuAtlasTextureExtent::new(u32::MAX as usize + 1, 1), None);
        assert_eq!(GpuAtlasTextureExtent::new(1, u32::MAX as usize + 1), None);
        assert_eq!(GpuAtlasTextureExtent::new(u32::MAX as usize, 1), None);
    }

    #[test]
    fn gpu_atlas_texture_extent_reports_upload_layout() {
        assert_eq!(
            GpuAtlasTextureExtent::new(8, 4),
            Some(GpuAtlasTextureExtent {
                width: 8,
                height: 4,
                bytes_per_row: 32,
            })
        );
    }
}
