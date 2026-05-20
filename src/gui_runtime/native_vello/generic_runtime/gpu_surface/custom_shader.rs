use crate::runtime::PaintGpuSurface;

use super::*;

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        _surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        stats.unsupported_custom_shader_surfaces += 1;
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
                descriptor: Arc::new(GpuShaderSurfaceDescriptor::new("test/custom-shader")),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        renderer.render_custom_shader(&surface, &mut stats);

        assert_eq!(stats.unsupported_custom_shader_surfaces, 1);
    }
}
