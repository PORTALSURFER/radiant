use crate::gui_runtime::native_vello::{
    generic_runtime::gpu_upload_bytes::{GpuUploadBytes, upload_slice_as_bytes},
    wgpu,
};

/// CPU-side vertex ABI shared by post-GPU overlay geometry and the WGPU pipeline.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(super) struct OverlayVertex {
    pub(super) position: [f32; 2],
    color: [f32; 4],
}

impl OverlayVertex {
    pub(super) const fn new(position: [f32; 2], color: [f32; 4]) -> Self {
        Self { position, color }
    }

    pub(super) fn position_attribute() -> wgpu::VertexAttribute {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        }
    }

    pub(super) fn color_attribute() -> wgpu::VertexAttribute {
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            shader_location: 1,
        }
    }
}

pub(super) fn overlay_vertex_bytes(vertices: &[OverlayVertex]) -> &[u8] {
    upload_slice_as_bytes(vertices)
}

unsafe impl GpuUploadBytes for OverlayVertex {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_vertices_keep_vertex_buffer_stride_stable() {
        assert_eq!(std::mem::size_of::<OverlayVertex>(), 24);
        assert_eq!(
            OverlayVertex::color_attribute().offset,
            std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
        );
    }

    #[test]
    fn overlay_vertex_bytes_cover_all_vertices() {
        let vertices = [
            OverlayVertex::new([0.0, 0.0], [1.0, 0.0, 0.0, 1.0]),
            OverlayVertex::new([1.0, 1.0], [0.0, 1.0, 0.0, 1.0]),
        ];

        assert_eq!(
            overlay_vertex_bytes(&vertices).len(),
            vertices.len() * std::mem::size_of::<OverlayVertex>()
        );
    }
}
