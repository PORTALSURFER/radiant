use vello::wgpu;

pub(super) fn wgpu_device_id(device: &wgpu::Device) -> usize {
    device as *const wgpu::Device as usize
}

pub(super) fn wgpu_target_matches(
    cached_device: usize,
    cached_format: wgpu::TextureFormat,
    target_device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
) -> bool {
    target_key_matches(
        cached_device,
        cached_format,
        wgpu_device_id(target_device),
        target_format,
    )
}

fn target_key_matches(
    cached_device: usize,
    cached_format: wgpu::TextureFormat,
    target_device: usize,
    target_format: wgpu::TextureFormat,
) -> bool {
    cached_device == target_device && cached_format == target_format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_key_tracks_device_and_format() {
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let target_device = 7;
        assert!(target_key_matches(7, format, target_device, format));
        assert!(!target_key_matches(7, format, 8, format));
        assert!(!target_key_matches(
            7,
            format,
            target_device,
            wgpu::TextureFormat::Rgba8UnormSrgb
        ));
    }
}
