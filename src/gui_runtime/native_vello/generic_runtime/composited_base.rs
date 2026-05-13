//! Cached composed frame used by paint-only transient overlay presentations.

use super::*;

pub(super) struct CompositedBaseFrame {
    _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl CompositedBaseFrame {
    pub(super) fn ensure<'a>(
        frame: &'a mut Option<Self>,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> &'a mut Self {
        if frame
            .as_ref()
            .is_none_or(|frame| !frame.matches(width, height, format))
        {
            *frame = Some(Self::new(device, width, height, format));
        }
        frame
            .as_mut()
            .expect("composited base frame is initialized")
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
        }
    }

    fn matches(&self, width: u32, height: u32, format: wgpu::TextureFormat) -> bool {
        composited_base_frame_matches_descriptor(
            self.width,
            self.height,
            self.format,
            width,
            height,
            format,
        )
    }
}

fn composited_base_frame_matches_descriptor(
    stored_width: u32,
    stored_height: u32,
    stored_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> bool {
    stored_width == width.max(1) && stored_height == height.max(1) && stored_format == format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composited_base_frame_matches_surface_descriptor() {
        assert!(composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm
        ));
        assert!(!composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            641,
            360,
            wgpu::TextureFormat::Bgra8Unorm
        ));
        assert!(!composited_base_frame_matches_descriptor(
            640,
            360,
            wgpu::TextureFormat::Bgra8Unorm,
            640,
            360,
            wgpu::TextureFormat::Rgba8Unorm
        ));
    }
}
