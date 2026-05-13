use super::target::PostGpuOverlayRenderTarget;
use crate::gui_runtime::native_vello::wgpu;

#[derive(Default)]
pub(super) struct OverlayVertexBuffer {
    buffer: Option<wgpu::Buffer>,
    capacity: wgpu::BufferAddress,
    device: usize,
}

impl OverlayVertexBuffer {
    pub(super) fn upload(
        &mut self,
        target: &PostGpuOverlayRenderTarget<'_>,
        vertex_bytes: &[u8],
    ) -> &wgpu::Buffer {
        let required_capacity = vertex_buffer_capacity_for(vertex_bytes.len());
        let device_id = target.device as *const wgpu::Device as usize;
        if self.needs_buffer(required_capacity, device_id) {
            self.buffer = Some(target.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("radiant_post_gpu_overlay_vertices"),
                size: required_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.capacity = required_capacity;
            self.device = device_id;
        }
        let buffer = self
            .buffer
            .as_ref()
            .expect("overlay vertex buffer is created before upload");
        target.queue.write_buffer(buffer, 0, vertex_bytes);
        buffer
    }

    fn needs_buffer(&self, required_capacity: wgpu::BufferAddress, device: usize) -> bool {
        needs_vertex_buffer(
            self.buffer.is_some(),
            self.capacity,
            self.device,
            required_capacity,
            device,
        )
    }
}

fn needs_vertex_buffer(
    has_buffer: bool,
    capacity: wgpu::BufferAddress,
    device: usize,
    required_capacity: wgpu::BufferAddress,
    required_device: usize,
) -> bool {
    !has_buffer || capacity < required_capacity || device != required_device
}

fn vertex_buffer_capacity_for(required_bytes: usize) -> wgpu::BufferAddress {
    required_bytes
        .checked_next_power_of_two()
        .unwrap_or(required_bytes)
        .max(1) as wgpu::BufferAddress
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_buffer_capacity_grows_by_powers_of_two() {
        assert_eq!(vertex_buffer_capacity_for(0), 1);
        assert_eq!(vertex_buffer_capacity_for(1), 1);
        assert_eq!(vertex_buffer_capacity_for(24), 32);
        assert_eq!(vertex_buffer_capacity_for(33), 64);
    }

    #[test]
    fn vertex_buffer_cache_reuses_matching_capacity_and_device() {
        assert!(!needs_vertex_buffer(true, 64, 7, 32, 7));
        assert!(needs_vertex_buffer(false, 64, 7, 32, 7));
        assert!(needs_vertex_buffer(true, 64, 7, 128, 7));
        assert!(needs_vertex_buffer(true, 64, 7, 32, 8));
    }
}
