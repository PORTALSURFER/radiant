use super::super::*;
use crate::runtime::GpuShaderSurfaceDescriptor;

pub(super) fn custom_shader_validation_error(device: &wgpu::Device) -> Option<wgpu::Error> {
    pollster::block_on(device.pop_error_scope())
}

pub(super) fn record_unsupported_custom_shader(
    descriptor: &GpuShaderSurfaceDescriptor,
    stats: &mut GpuSurfaceRenderStats,
) {
    stats.unsupported_custom_shader_surfaces += 1;
    stats.unsupported_custom_shader_vertices += descriptor.vertex_count as usize;
    stats.unsupported_custom_shader_source_bytes += descriptor
        .wgsl_source
        .as_ref()
        .map_or(0, |source| source.len());
    stats.unsupported_custom_shader_uniform_bytes += descriptor.uniform_bytes.len();
    stats.unsupported_custom_shader_storage_bytes += descriptor.storage_bytes.len();
}

pub(super) fn record_failed_custom_shader_surface(stats: &mut GpuSurfaceRenderStats) {
    stats.custom_shader_surfaces_failed += 1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{Point, Rect, Vector2},
        runtime::{GpuSurfaceCapabilities, GpuSurfaceContent},
    };

    #[test]
    fn custom_shader_unsupported_diagnostics_count_payload_bytes() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0)),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(
                    GpuShaderSurfaceDescriptor::new("test/custom-shader")
                        .wgsl_source(
                            "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
                        )
                        .entry_point("vertex_main")
                        .fragment_entry_point("fragment_main")
                        .uniform_bytes([1, 2, 3, 4])
                        .storage_bytes([5, 6, 7])
                        .vertex_count(6),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        record_unsupported_custom_shader(
            match &surface.content {
                GpuSurfaceContent::CustomShader { descriptor } => descriptor,
                _ => unreachable!(),
            },
            &mut stats,
        );

        assert_eq!(stats.unsupported_custom_shader_surfaces, 1);
        assert_eq!(stats.unsupported_custom_shader_vertices, 6);
        assert!(stats.unsupported_custom_shader_source_bytes > 0);
        assert_eq!(stats.unsupported_custom_shader_uniform_bytes, 4);
        assert_eq!(stats.unsupported_custom_shader_storage_bytes, 3);
    }
}
