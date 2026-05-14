use super::*;

pub(super) struct GpuSurfacePipeline {
    pub(super) format: wgpu::TextureFormat,
    pub(super) device: usize,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) sampler: wgpu::Sampler,
}

impl GpuSurfacePipeline {
    pub(super) fn matches_target(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> bool {
        pipeline_matches_target(self.device, self.format, device_id(device), format)
    }
}

pub(super) struct GpuSurfaceTexture {
    pub(super) revision: u64,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

impl GpuSurfaceTexture {
    pub(super) fn matches_atlas(&self, revision: u64, width: usize, height: usize) -> bool {
        atlas_texture_matches_descriptor(
            self.revision,
            self.width,
            self.height,
            revision,
            width,
            height,
        )
    }
}

fn atlas_texture_matches_descriptor(
    stored_revision: u64,
    stored_width: usize,
    stored_height: usize,
    revision: u64,
    width: usize,
    height: usize,
) -> bool {
    stored_revision == revision && stored_width == width && stored_height == height
}

pub(super) struct GpuSurfaceCompositeBinding {
    pub(super) cache_key: GpuSurfaceCompositeBindingKey,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuSurfaceCompositeBindingKey {
    pub(super) pipeline_generation: u64,
    pub(super) texture: GpuSurfaceTextureIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuSurfaceTextureIdentity {
    RgbaAtlas {
        revision: u64,
        width: usize,
        height: usize,
    },
    SignalBody(SignalBodyCacheKey),
}

pub(super) struct SignalPipeline {
    pub(super) format: wgpu::TextureFormat,
    pub(super) device: usize,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) pipeline: wgpu::RenderPipeline,
}

impl SignalPipeline {
    pub(super) fn matches_target(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> bool {
        pipeline_matches_target(self.device, self.format, device_id(device), format)
    }
}

pub(super) struct SignalBuffer {
    pub(super) cache_key: SignalBufferCacheKey,
    pub(super) sample_count: usize,
    pub(super) pipeline_generation: u64,
    pub(super) _sample_buffer: wgpu::Buffer,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SignalBufferCacheKey {
    pub(super) revision: u64,
    pub(super) level_index: usize,
    pub(super) style_revision: u32,
}

impl SignalBufferCacheKey {
    pub(super) fn new(revision: u64, level_index: usize) -> Self {
        Self {
            revision,
            level_index,
            style_revision: GPU_SIGNAL_STYLE_REVISION,
        }
    }
}

pub(super) struct CachedSignalSummary {
    pub(super) revision: u64,
    pub(super) frames: usize,
    pub(super) band_count: usize,
    pub(super) sample_count: usize,
    pub(super) summary: Arc<GpuSignalSummary>,
}

pub(super) struct SignalBodyTexture {
    pub(super) cache_key: SignalBodyCacheKey,
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SignalBodyCacheKey {
    pub(super) revision: u64,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) frame_start_bits: u32,
    pub(super) frame_end_bits: u32,
    pub(super) frames: usize,
    pub(super) band_count: usize,
    pub(super) sample_count: usize,
    pub(super) level_index: usize,
    pub(super) style_revision: u32,
}

impl SignalBodyCacheKey {
    pub(super) fn new(
        surface: &PaintGpuSurface,
        frames: usize,
        band_count: usize,
        frame_range: [f32; 2],
        sample_count: usize,
        level_index: usize,
    ) -> Self {
        Self {
            revision: surface.revision,
            width: surface.rect.width().ceil().max(1.0) as u32,
            height: surface.rect.height().ceil().max(1.0) as u32,
            frame_start_bits: frame_range[0].to_bits(),
            frame_end_bits: frame_range[1].to_bits(),
            frames,
            band_count,
            sample_count,
            level_index,
            style_revision: GPU_SIGNAL_STYLE_REVISION,
        }
    }
}

const GPU_SIGNAL_STYLE_REVISION: u32 = 1;

pub(super) fn device_id(device: &wgpu::Device) -> usize {
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct GpuSurfaceUniforms {
    pub(super) dest: [f32; 4],
    pub(super) source: [f32; 4],
    pub(super) target_size: [f32; 2],
    pub(super) _padding: [f32; 2],
    pub(super) overlay_ratios: [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    pub(super) overlay_widths: [[f32; 4]; GPU_SURFACE_OVERLAY_VEC4_SLOTS],
    pub(super) overlay_colors: [[f32; 4]; MAX_GPU_SURFACE_OVERLAYS],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SignalUniforms {
    pub(super) dest: [f32; 4],
    pub(super) frame_range: [f32; 4],
    pub(super) summary_meta: [f32; 4],
    pub(super) target_size: [f32; 2],
    pub(super) cursor_ratio: f32,
    pub(super) cursor_width: f32,
    pub(super) cursor_color: [f32; 4],
}

pub(super) const MAX_GPU_SURFACE_OVERLAYS: usize = 8;
pub(super) const GPU_SURFACE_OVERLAY_VEC4_SLOTS: usize = MAX_GPU_SURFACE_OVERLAYS / 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_buffer_cache_key_keeps_revision_and_level_independent() {
        let high_revision = SignalBufferCacheKey::new(1_u64 << 32, 0);
        let low_revision_high_level = SignalBufferCacheKey::new(0, 1);

        assert_ne!(high_revision, low_revision_high_level);
    }

    #[test]
    fn composite_binding_key_tracks_pipeline_and_texture_identity() {
        let atlas = GpuSurfaceCompositeBindingKey {
            pipeline_generation: 1,
            texture: GpuSurfaceTextureIdentity::RgbaAtlas {
                revision: 7,
                width: 64,
                height: 32,
            },
        };
        let same_texture_next_pipeline = GpuSurfaceCompositeBindingKey {
            pipeline_generation: 2,
            ..atlas
        };
        let next_texture = GpuSurfaceCompositeBindingKey {
            pipeline_generation: 1,
            texture: GpuSurfaceTextureIdentity::RgbaAtlas {
                revision: 8,
                width: 64,
                height: 32,
            },
        };

        assert_ne!(atlas, same_texture_next_pipeline);
        assert_ne!(atlas, next_texture);
    }

    #[test]
    fn atlas_texture_identity_tracks_revision_and_dimensions() {
        assert!(atlas_texture_matches_descriptor(7, 64, 32, 7, 64, 32));
        assert!(!atlas_texture_matches_descriptor(7, 64, 32, 8, 64, 32));
        assert!(!atlas_texture_matches_descriptor(7, 64, 32, 7, 65, 32));
        assert!(!atlas_texture_matches_descriptor(7, 64, 32, 7, 64, 33));
    }

    #[test]
    fn pipeline_cache_key_tracks_device_and_format() {
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
