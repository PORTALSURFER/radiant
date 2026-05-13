//! Immediate shape replay for paint ordered above native GPU surfaces.

mod geometry;
mod pipeline;
mod target;

use crate::gui_runtime::native_vello::wgpu;
use geometry::{OverlayVertex, overlay_vertex_bytes, replayable_vertices_into};
use pipeline::PostGpuOverlayPipeline;
pub(in crate::gui_runtime::native_vello::generic_runtime) use target::PostGpuOverlayRenderTarget;

#[derive(Default)]
pub(super) struct PostGpuOverlayRenderer {
    pipeline: Option<PostGpuOverlayPipeline>,
    vertices: Vec<OverlayVertex>,
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_buffer_capacity: wgpu::BufferAddress,
    vertex_buffer_device: usize,
}

impl PostGpuOverlayRenderer {
    pub(super) fn render(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[crate::runtime::PaintPrimitive],
    ) {
        let Some(suffix) = geometry::replayable_suffix(primitives) else {
            return;
        };
        self.render_primitives(target, suffix);
    }

    pub(super) fn render_primitives(
        &mut self,
        target: &mut PostGpuOverlayRenderTarget<'_>,
        primitives: &[crate::runtime::PaintPrimitive],
    ) {
        replayable_vertices_into(primitives, target.size, &mut self.vertices);
        if self.vertices.is_empty() {
            return;
        }
        let vertex_bytes = overlay_vertex_bytes(&self.vertices);
        upload_vertex_bytes(
            target,
            &mut self.vertex_buffer,
            &mut self.vertex_buffer_capacity,
            &mut self.vertex_buffer_device,
            vertex_bytes,
        );
        let pipeline = self
            .pipeline
            .get_or_insert_with(|| PostGpuOverlayPipeline::new(target.device, target.format));
        if pipeline.format() != target.format {
            *pipeline = PostGpuOverlayPipeline::new(target.device, target.format);
        }
        let vertex_buffer = self
            .vertex_buffer
            .as_ref()
            .expect("overlay vertex buffer is created before rendering");
        pipeline.render(target, vertex_buffer, self.vertices.len() as u32);
    }
}

fn upload_vertex_bytes(
    target: &PostGpuOverlayRenderTarget<'_>,
    vertex_buffer: &mut Option<wgpu::Buffer>,
    vertex_buffer_capacity: &mut wgpu::BufferAddress,
    vertex_buffer_device: &mut usize,
    vertex_bytes: &[u8],
) {
    let required_capacity = vertex_buffer_capacity_for(vertex_bytes.len());
    let device_id = target.device as *const wgpu::Device as usize;
    let needs_buffer = vertex_buffer.is_none()
        || *vertex_buffer_capacity < required_capacity
        || *vertex_buffer_device != device_id;
    if needs_buffer {
        *vertex_buffer = Some(target.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("radiant_post_gpu_overlay_vertices"),
            size: required_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        *vertex_buffer_capacity = required_capacity;
        *vertex_buffer_device = device_id;
    }
    if let Some(vertex_buffer) = vertex_buffer.as_ref() {
        target.queue.write_buffer(vertex_buffer, 0, vertex_bytes);
    }
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
}
