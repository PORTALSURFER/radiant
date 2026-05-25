use super::super::device::wgpu_device_id;
use vello::wgpu;

pub(in crate::gui_runtime::native_vello::generic_runtime) struct CompositedBaseFrame {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    device: usize,
}

impl CompositedBaseFrame {
    pub(super) fn ensure<'a>(
        frame: &'a mut Option<Self>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (&'a mut Self, bool) {
        if frame
            .as_ref()
            .is_some_and(|frame| frame.matches(device, width, height, format))
            && let Some(existing) = frame
        {
            return (existing, false);
        }
        (frame.insert(Self::new(device, width, height, format)), true)
    }

    fn new(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("radiant_composited_base_frame"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
            width: width.max(1),
            height: height.max(1),
            format,
            device: wgpu_device_id(device),
        }
    }

    fn matches(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> bool {
        composited_base_frame_matches_descriptor(
            CompositedBaseFrameDescriptor {
                device: self.device,
                width: self.width,
                height: self.height,
                format: self.format,
            },
            CompositedBaseFrameDescriptor {
                device: wgpu_device_id(device),
                width: width.max(1),
                height: height.max(1),
                format,
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompositedBaseFrameDescriptor {
    device: usize,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

fn composited_base_frame_matches_descriptor(
    stored: CompositedBaseFrameDescriptor,
    target: CompositedBaseFrameDescriptor,
) -> bool {
    stored == target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composited_base_frame_matches_surface_descriptor() {
        let descriptor = CompositedBaseFrameDescriptor {
            device: 7,
            width: 640,
            height: 360,
            format: wgpu::TextureFormat::Bgra8Unorm,
        };
        assert!(composited_base_frame_matches_descriptor(
            descriptor, descriptor
        ));
        assert!(!composited_base_frame_matches_descriptor(
            descriptor,
            CompositedBaseFrameDescriptor {
                device: 8,
                ..descriptor
            }
        ));
        assert!(!composited_base_frame_matches_descriptor(
            descriptor,
            CompositedBaseFrameDescriptor {
                width: 641,
                ..descriptor
            }
        ));
        assert!(!composited_base_frame_matches_descriptor(
            descriptor,
            CompositedBaseFrameDescriptor {
                format: wgpu::TextureFormat::Rgba8Unorm,
                ..descriptor
            }
        ));
    }
}
