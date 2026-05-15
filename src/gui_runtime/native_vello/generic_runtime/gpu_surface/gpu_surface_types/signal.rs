use super::*;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalBuffer {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cache_key:
        SignalBufferCacheKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) sample_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline_generation: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _sample_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) uniform_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group:
        wgpu::BindGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalBufferCacheKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) level_index: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) style_revision: u32,
}

impl SignalBufferCacheKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn new(
        revision: u64,
        level_index: usize,
    ) -> Self {
        Self {
            revision,
            level_index,
            style_revision: GPU_SIGNAL_STYLE_REVISION,
        }
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct CachedSignalSummary {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frames: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) band_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) sample_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) summary:
        Arc<GpuSignalSummary>,
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalBodyTexture {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cache_key:
        SignalBodyCacheKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _texture: wgpu::Texture,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) view: wgpu::TextureView,
}

impl SignalBodyTexture {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn matches_body(
        &self,
        device: &wgpu::Device,
        cache_key: SignalBodyCacheKey,
    ) -> bool {
        signal_body_matches_key(
            self.device,
            self.cache_key,
            wgpu_device_id(device),
            cache_key,
        )
    }
}

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalBodyCacheKeyParts
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) extent:
        SurfacePixelExtent,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frames: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) band_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frame_range: [f32; 2],
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) sample_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) level_index: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) gain_preview:
        Option<GpuSignalGainPreview>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalBodyCacheKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) width: u32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) height: u32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frame_start_bits: u32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frame_end_bits: u32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) frames: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) band_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) sample_count: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) level_index: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) style_revision: u32,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) gain_preview:
        SignalGainPreviewKey,
}

impl SignalBodyCacheKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn new(
        parts: SignalBodyCacheKeyParts,
    ) -> Self {
        Self {
            revision: parts.revision,
            width: parts.extent.width,
            height: parts.extent.height,
            frame_start_bits: parts.frame_range[0].to_bits(),
            frame_end_bits: parts.frame_range[1].to_bits(),
            frames: parts.frames,
            band_count: parts.band_count,
            sample_count: parts.sample_count,
            level_index: parts.level_index,
            style_revision: GPU_SIGNAL_STYLE_REVISION,
            gain_preview: SignalGainPreviewKey::new(parts.gain_preview),
        }
    }
}

const GPU_SIGNAL_STYLE_REVISION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct SignalGainPreviewKey {
    bits: [u32; 12],
}

impl SignalGainPreviewKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn new(
        preview: Option<GpuSignalGainPreview>,
    ) -> Self {
        let Some(preview) = preview else {
            return Self { bits: [0; 12] };
        };
        Self {
            bits: [
                1,
                preview.start.to_bits(),
                preview.end.to_bits(),
                preview.gain.to_bits(),
                preview.fade_in_length.to_bits(),
                preview.fade_in_curve.to_bits(),
                preview.fade_in_mute.to_bits(),
                preview.fade_out_length.to_bits(),
                preview.fade_out_curve.to_bits(),
                preview.fade_out_mute.to_bits(),
                0,
                0,
            ],
        }
    }
}

fn signal_body_matches_key(
    cached_device: usize,
    cached_key: SignalBodyCacheKey,
    target_device: usize,
    target_key: SignalBodyCacheKey,
) -> bool {
    cached_device == target_device && cached_key == target_key
}

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
    fn signal_body_texture_identity_tracks_device_and_body_key() {
        let key = SignalBodyCacheKey {
            revision: 1,
            width: 64,
            height: 32,
            frame_start_bits: 0.0f32.to_bits(),
            frame_end_bits: 1.0f32.to_bits(),
            frames: 128,
            band_count: 2,
            sample_count: 256,
            level_index: 0,
            style_revision: 1,
            gain_preview: SignalGainPreviewKey::new(None),
        };
        let next_key = SignalBodyCacheKey { revision: 2, ..key };

        assert!(signal_body_matches_key(7, key, 7, key));
        assert!(!signal_body_matches_key(7, key, 8, key));
        assert!(!signal_body_matches_key(7, key, 7, next_key));
    }

    #[test]
    fn signal_gain_preview_key_tracks_preview_parameters() {
        let preview = GpuSignalGainPreview {
            start: 0.1,
            end: 0.8,
            gain: 0.75,
            fade_in_length: 0.25,
            fade_in_curve: 0.4,
            fade_in_mute: 0.0,
            fade_out_length: 0.2,
            fade_out_curve: 0.6,
            fade_out_mute: 0.1,
        };
        let mut changed = preview;
        changed.fade_in_length = 0.3;

        assert_ne!(
            SignalGainPreviewKey::new(None),
            SignalGainPreviewKey::new(Some(preview))
        );
        assert_ne!(
            SignalGainPreviewKey::new(Some(preview)),
            SignalGainPreviewKey::new(Some(changed))
        );
    }
}
