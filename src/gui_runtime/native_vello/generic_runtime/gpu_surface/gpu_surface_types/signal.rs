use super::*;

mod cache_key;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) use cache_key::{
    SignalBodyCacheKey, SignalBodyCacheKeyParts, SignalBufferCacheKey, signal_body_matches_key,
};

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

#[cfg(test)]
mod tests {
    use super::*;

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
            gain_preview: cache_key::SignalGainPreviewKey::new(None),
        };
        let next_key = SignalBodyCacheKey { revision: 2, ..key };

        assert!(signal_body_matches_key(7, key, 7, key));
        assert!(!signal_body_matches_key(7, key, 8, key));
        assert!(!signal_body_matches_key(7, key, 7, next_key));
    }
}
