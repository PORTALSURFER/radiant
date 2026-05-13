use crate::gui_runtime::native_vello::wgpu;

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
    let size = std::mem::size_of_val(vertices);
    let ptr = vertices.as_ptr() as *const u8;
    // SAFETY: `OverlayVertex` is a repr(C) POD-like value containing only f32
    // arrays. The slice is used only while wgpu copies the bytes into a buffer.
    unsafe { std::slice::from_raw_parts(ptr, size) }
}

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
