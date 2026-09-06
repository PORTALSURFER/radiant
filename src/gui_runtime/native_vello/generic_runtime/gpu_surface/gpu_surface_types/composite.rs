use super::super::identity::RenderCanvasContentIdentity;
use super::super::identity::RenderCanvasContentOwner;
use super::signal::SignalBodyCacheKey;
use crate::gui_runtime::native_vello::generic_runtime::signal_summary_prepare::SignalGpuLease;
use std::sync::Arc;
use vello::wgpu;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceCompositeBinding
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) cache_key:
        GpuSurfaceCompositeBindingKey,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) uniform_buffer:
        wgpu::Buffer,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) bind_group:
        wgpu::BindGroup,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _signal_owner:
        Option<RenderCanvasContentOwner>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _signal_body_lease:
        Option<Arc<SignalGpuLease>>,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _signal_uniform_lease:
        Option<SignalGpuLease>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceCompositeBindingKey
{
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) pipeline_generation: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) texture:
        GpuSurfaceTextureIdentity,
}

impl GpuSurfaceCompositeBindingKey {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn revision(
        self,
    ) -> u64 {
        self.texture.revision()
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) enum GpuSurfaceTextureIdentity
{
    RgbaAtlas {
        revision: u64,
        content_identity: RenderCanvasContentIdentity,
        width: usize,
        height: usize,
    },
    SignalBody(SignalBodyCacheKey),
}

impl GpuSurfaceTextureIdentity {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn revision(
        self,
    ) -> u64 {
        match self {
            Self::RgbaAtlas { revision, .. } => revision,
            Self::SignalBody(key) => key.revision,
        }
    }
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
                content_identity: crate::gui_runtime::native_vello::generic_runtime::gpu_surface::identity::RenderCanvasContentIdentity::CustomShader { descriptor: 1 },
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
                content_identity: crate::gui_runtime::native_vello::generic_runtime::gpu_surface::identity::RenderCanvasContentIdentity::CustomShader { descriptor: 1 },
                width: 64,
                height: 32,
            },
        };

        assert_ne!(atlas, same_texture_next_pipeline);
        assert_ne!(atlas, next_texture);
    }
}
