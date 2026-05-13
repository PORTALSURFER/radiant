use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GpuSurfaceRenderStats {
    pub(crate) atlas_texture_uploads: usize,
    pub(crate) signal_summary_builds: usize,
    pub(crate) signal_summary_cache_hits: usize,
    pub(crate) signal_body_renders: usize,
    pub(crate) signal_body_cache_hits: usize,
    pub(crate) signal_body_encode_elapsed: Duration,
    pub(crate) composite_encode_elapsed: Duration,
}

pub(super) struct GpuSurfacePipeline {
    pub(super) format: wgpu::TextureFormat,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) sampler: wgpu::Sampler,
}

pub(super) struct GpuSurfaceTexture {
    pub(super) revision: u64,
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

pub(super) struct SignalPipeline {
    pub(super) format: wgpu::TextureFormat,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub(super) pipeline: wgpu::RenderPipeline,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}
