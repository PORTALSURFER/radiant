use super::super::wgpu_device_id;
use vello::wgpu;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) struct GpuSurfaceTexture {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) device: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) revision: u64,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) width: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) height: usize,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) _texture: wgpu::Texture,
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) view: wgpu::TextureView,
}

impl GpuSurfaceTexture {
    pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn matches_atlas(
        &self,
        device: &wgpu::Device,
        revision: u64,
        width: usize,
        height: usize,
    ) -> bool {
        atlas_texture_matches_descriptor(
            AtlasTextureDescriptor {
                device: self.device,
                revision: self.revision,
                width: self.width,
                height: self.height,
            },
            AtlasTextureDescriptor {
                device: wgpu_device_id(device),
                revision,
                width,
                height,
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AtlasTextureDescriptor {
    device: usize,
    revision: u64,
    width: usize,
    height: usize,
}

fn atlas_texture_matches_descriptor(
    stored: AtlasTextureDescriptor,
    target: AtlasTextureDescriptor,
) -> bool {
    stored == target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_texture_identity_tracks_device_revision_and_dimensions() {
        let descriptor = AtlasTextureDescriptor {
            device: 7,
            revision: 8,
            width: 64,
            height: 32,
        };
        assert!(atlas_texture_matches_descriptor(descriptor, descriptor));
        assert!(!atlas_texture_matches_descriptor(
            descriptor,
            AtlasTextureDescriptor {
                device: 6,
                ..descriptor
            }
        ));
        assert!(!atlas_texture_matches_descriptor(
            descriptor,
            AtlasTextureDescriptor {
                revision: 9,
                ..descriptor
            }
        ));
        assert!(!atlas_texture_matches_descriptor(
            descriptor,
            AtlasTextureDescriptor {
                width: 65,
                ..descriptor
            }
        ));
        assert!(!atlas_texture_matches_descriptor(
            descriptor,
            AtlasTextureDescriptor {
                height: 33,
                ..descriptor
            }
        ));
    }
}
