use crate::runtime::PaintGpuSurface;

use super::*;

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        stats.unsupported_custom_shader_surfaces += 1;
        if let crate::runtime::GpuSurfaceContent::CustomShader { descriptor } = &surface.content {
            stats.unsupported_custom_shader_vertices += descriptor.vertex_count as usize;
            stats.unsupported_custom_shader_source_bytes += descriptor
                .wgsl_source
                .as_ref()
                .map_or(0, |source| source.len());
            stats.unsupported_custom_shader_uniform_bytes += descriptor.uniform_bytes.len();
            stats.unsupported_custom_shader_storage_bytes += descriptor.storage_bytes.len();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{Point, Rect, Vector2},
        runtime::{GpuShaderSurfaceDescriptor, GpuSurfaceCapabilities, GpuSurfaceContent},
    };
    use std::sync::Arc;

    #[test]
    fn custom_shader_surfaces_report_unsupported_until_pipeline_exists() {
        let mut renderer = GpuSurfaceRenderer::default();
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
                            "@vertex fn main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }",
                        )
                        .uniform_bytes([1, 2, 3, 4])
                        .storage_bytes([5, 6, 7])
                        .vertex_count(6),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        renderer.render_custom_shader(&surface, &mut stats);

        assert_eq!(stats.unsupported_custom_shader_surfaces, 1);
        assert_eq!(stats.unsupported_custom_shader_vertices, 6);
        assert!(stats.unsupported_custom_shader_source_bytes > 0);
        assert_eq!(stats.unsupported_custom_shader_uniform_bytes, 4);
        assert_eq!(stats.unsupported_custom_shader_storage_bytes, 3);
    }
}
