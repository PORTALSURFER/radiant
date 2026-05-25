use super::signal::SignalBodyCacheKey;
use vello::wgpu;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceCompositeBinding
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cache_key:
        GpuSurfaceCompositeBindingKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) uniform_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group:
        wgpu::BindGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceCompositeBindingKey
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline_generation: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) texture:
        GpuSurfaceTextureIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) enum GpuSurfaceTextureIdentity
{
    RgbaAtlas {
        revision: u64,
        width: usize,
        height: usize,
    },
    SignalBody(SignalBodyCacheKey),
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
