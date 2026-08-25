use super::super::gpu_surface_types::GpuSurfaceTexture;
use super::super::identity::{RenderCanvasContentIdentity, RenderCanvasContentOwner};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::upload_plan::{
    GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
    GpuSurfaceRenderCanvasUploadAtlasTextureOperation,
    GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
};
use super::super::{GpuSurfaceRenderer, wgpu_device_id};
use crate::runtime::{GpuSurfaceContent, PaintGpuSurface};
use vello::wgpu;

impl GpuSurfaceRenderer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn ensure_texture_legacy(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &surface.content else {
            return;
        };
        let content_identity = RenderCanvasContentIdentity::from_content(&surface.content);
        let device_id = wgpu_device_id(device);
        if let Some(texture) = self.resources.textures.get(&surface.key) {
            if texture.matches_atlas_identity(
                device_id,
                surface.revision,
                content_identity,
                atlas.width(),
                atlas.height(),
            ) {
                stats.atlas.texture_cache_hits += 1;
                return;
            }
            if texture.revision != surface.revision {
                stats.atlas.texture_revision_mismatches += 1;
            } else {
                stats.atlas.texture_content_mismatches += 1;
            }
        }
        let extent = match GpuAtlasTextureExtent::try_new(
            atlas.width(),
            atlas.height(),
            atlas.pixels().len(),
        ) {
            Ok(extent) => extent,
            Err(reason) => {
                stats.mark_candidate_unavailable(reason);
                return;
            }
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
        let pixels = atlas.pixels();
        stats.record_candidate_immutable_payload(pixels.len());
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixels,
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
        record_atlas_texture_upload(stats, pixels.len());
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.resources.textures.insert(
            surface.key,
            GpuSurfaceTexture {
                device: device_id,
                revision: surface.revision,
                content_identity,
                _content_owner: RenderCanvasContentOwner::from_content(&surface.content),
                width: atlas.width(),
                height: atlas.height(),
                _texture: texture,
                view,
            },
            atlas.width(),
            atlas.height(),
        );
        stats.atlas.texture_uploads += 1;
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn preflight_atlas_texture(
        &self,
        device: usize,
        surface_index: usize,
        surface: &PaintGpuSurface,
    ) -> Result<
        GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
        GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    > {
        let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &surface.content else {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid);
        };
        let content_identity = RenderCanvasContentIdentity::from_content(&surface.content);
        let extent =
            GpuAtlasTextureExtent::try_new(atlas.width(), atlas.height(), atlas.pixels().len())?;
        Ok(GpuSurfaceRenderCanvasUploadAtlasTextureExecution {
            surface_index,
            key: surface.key,
            device,
            revision: surface.revision,
            content_identity,
            width: atlas.width(),
            height: atlas.height(),
            extent_width: extent.width,
            extent_height: extent.height,
            bytes_per_row: extent.bytes_per_row,
            byte_len: atlas.pixels().len(),
            operation: atlas_texture_operation(
                self.resources.textures.get(&surface.key),
                device,
                surface.revision,
                content_identity,
                atlas.width(),
                atlas.height(),
            ),
        })
    }

    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn execute_atlas_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &PaintGpuSurface,
        execution: GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
        upload_byte_len: Option<usize>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &surface.content else {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid);
        };
        let content_identity = RenderCanvasContentIdentity::from_content(&surface.content);
        let extent =
            GpuAtlasTextureExtent::try_new(atlas.width(), atlas.height(), atlas.pixels().len())?;
        if execution.surface_index == usize::MAX
            || execution.key != surface.key
            || execution.device != wgpu_device_id(device)
            || execution.revision != surface.revision
            || execution.content_identity != content_identity
            || execution.width != atlas.width()
            || execution.height != atlas.height()
            || execution.extent_width != extent.width
            || execution.extent_height != extent.height
            || execution.bytes_per_row != extent.bytes_per_row
            || execution.byte_len != atlas.pixels().len()
        {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }

        let expected_operation = atlas_texture_operation(
            self.resources.textures.get(&surface.key),
            execution.device,
            surface.revision,
            content_identity,
            atlas.width(),
            atlas.height(),
        );
        if execution.operation != expected_operation {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }

        if let GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
            revision_mismatch,
            content_mismatch,
        } = expected_operation
        {
            if revision_mismatch {
                stats.atlas.texture_revision_mismatches += 1;
            } else if content_mismatch {
                stats.atlas.texture_content_mismatches += 1;
            }
        }

        match execution.operation {
            GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Reuse => {
                if upload_byte_len.is_some() {
                    return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                }
                stats.atlas.texture_cache_hits += 1;
                Ok(())
            }
            GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload { .. } => {
                let Some(upload_byte_len) = upload_byte_len else {
                    return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                };
                if upload_byte_len != execution.byte_len {
                    return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                }
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
                    atlas.pixels(),
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
                stats.record_candidate_immutable_payload(upload_byte_len);
                record_atlas_texture_upload(stats, upload_byte_len);
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                self.resources.textures.insert(
                    surface.key,
                    GpuSurfaceTexture {
                        device: execution.device,
                        revision: surface.revision,
                        content_identity,
                        _content_owner: RenderCanvasContentOwner::from_content(&surface.content),
                        width: atlas.width(),
                        height: atlas.height(),
                        _texture: texture,
                        view,
                    },
                    atlas.width(),
                    atlas.height(),
                );
                stats.atlas.texture_uploads += 1;
                Ok(())
            }
        }
    }
}

fn record_atlas_texture_upload(stats: &mut GpuSurfaceRenderStats, byte_len: usize) {
    stats
        .render_canvas_uploads
        .record_immutable_payload(byte_len);
}

fn atlas_texture_operation(
    cached: Option<&GpuSurfaceTexture>,
    device: usize,
    revision: u64,
    content_identity: RenderCanvasContentIdentity,
    width: usize,
    height: usize,
) -> GpuSurfaceRenderCanvasUploadAtlasTextureOperation {
    match cached {
        Some(texture)
            if texture.matches_atlas_identity(
                device,
                revision,
                content_identity,
                width,
                height,
            ) =>
        {
            GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Reuse
        }
        Some(texture) => GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
            revision_mismatch: texture.revision != revision,
            content_mismatch: texture.revision == revision,
        },
        None => GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
            revision_mismatch: false,
            content_mismatch: false,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuAtlasTextureExtent {
    width: u32,
    height: u32,
    bytes_per_row: u32,
}

impl GpuAtlasTextureExtent {
    #[cfg(test)]
    fn new(width: usize, height: usize, byte_len: usize) -> Option<Self> {
        Self::try_new(width, height, byte_len).ok()
    }

    pub(super) fn try_new(
        width: usize,
        height: usize,
        byte_len: usize,
    ) -> Result<Self, GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        let expected_byte_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
        if byte_len != expected_byte_len {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        let width = u32::try_from(width)
            .map_err(|_| GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
        let height = u32::try_from(height)
            .map_err(|_| GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
        if width == 0 || height == 0 {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        let bytes_per_row = width
            .checked_mul(4)
            .ok_or(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Overflow)?;
        Ok(Self {
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
        assert_eq!(GpuAtlasTextureExtent::new(0, 1, 0), None);
        assert_eq!(GpuAtlasTextureExtent::new(1, 0, 0), None);
        assert_eq!(
            GpuAtlasTextureExtent::new(u32::MAX as usize + 1, 1, (u32::MAX as usize + 1) * 4),
            None
        );
        assert_eq!(
            GpuAtlasTextureExtent::new(1, u32::MAX as usize + 1, (u32::MAX as usize + 1) * 4),
            None
        );
        assert_eq!(GpuAtlasTextureExtent::new(u32::MAX as usize, 1, 0), None);
    }

    #[test]
    fn gpu_atlas_texture_extent_rejects_short_long_and_overflowing_payloads() {
        assert_eq!(GpuAtlasTextureExtent::new(2, 2, 15), None);
        assert_eq!(GpuAtlasTextureExtent::new(2, 2, 17), None);
        assert_eq!(GpuAtlasTextureExtent::new(usize::MAX, 2, 0), None);
    }

    #[test]
    fn gpu_atlas_texture_extent_reports_upload_layout() {
        assert_eq!(
            GpuAtlasTextureExtent::new(8, 4, 8 * 4 * 4),
            Some(GpuAtlasTextureExtent {
                width: 8,
                height: 4,
                bytes_per_row: 32,
            })
        );
    }
}
