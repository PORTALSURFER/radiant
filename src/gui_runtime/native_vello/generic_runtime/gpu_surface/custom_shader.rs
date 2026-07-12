use super::stats::GpuSurfaceRenderStats;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{GpuShaderSurfaceDescriptor, GpuSurfaceContent, PaintGpuSurface};
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/draw.rs"]
mod draw;
#[path = "custom_shader/pipeline.rs"]
mod pipeline;
use binding::CustomShaderBindingRequest;
use diagnostics::{record_failed_custom_shader_surface, record_unsupported_custom_shader};
use draw::{CustomShaderBufferUploadRequest, CustomShaderDrawRequest};
use pipeline::{CustomShaderPipelineRequest, custom_shader_pipeline_key};

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(descriptor) = supported_custom_shader_descriptor(surface, stats) else {
            return;
        };
        if !self.prepare_custom_shader_resources(target, surface, descriptor, stats) {
            return;
        }
        let Some(pipeline) = self.resources.custom_shader_pipelines.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        draw::upload_custom_shader_buffers(CustomShaderBufferUploadRequest {
            target,
            surface,
            descriptor,
            binding,
        });
        draw::encode_custom_shader_draw(
            CustomShaderDrawRequest {
                target,
                surface,
                descriptor,
                pipeline,
                binding,
                occlusion_regions,
            },
            stats,
        );
        stats.custom_shader.surfaces_rendered += 1;
    }

    fn prepare_custom_shader_resources(
        &mut self,
        target: &GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        descriptor: &GpuShaderSurfaceDescriptor,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            record_unsupported_custom_shader(descriptor, stats);
            return false;
        };
        self.ensure_custom_shader_pipeline(
            CustomShaderPipelineRequest {
                surface_key: surface.key,
                device: target.device,
                target_format: target.format,
                key: pipeline_key,
            },
            stats,
        );
        if !self
            .resources
            .custom_shader_pipelines
            .contains_key(&surface.key)
        {
            record_failed_custom_shader_surface(stats);
            return false;
        }
        self.ensure_custom_shader_binding(
            CustomShaderBindingRequest {
                device: target.device,
                surface_key: surface.key,
                descriptor,
            },
            stats,
        );
        if self
            .resources
            .custom_shader_bindings
            .contains_key(&surface.key)
        {
            true
        } else {
            record_failed_custom_shader_surface(stats);
            false
        }
    }
}

fn supported_custom_shader_descriptor<'a>(
    surface: &'a PaintGpuSurface,
    stats: &mut GpuSurfaceRenderStats,
) -> Option<&'a GpuShaderSurfaceDescriptor> {
    let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
        return None;
    };
    if !custom_shader_descriptor_is_supported(descriptor) {
        record_unsupported_custom_shader(descriptor, stats);
        return None;
    }
    Some(descriptor)
}

pub(super) fn custom_shader_descriptor_is_supported(
    descriptor: &GpuShaderSurfaceDescriptor,
) -> bool {
    descriptor.wgsl_source.is_some() && descriptor.fragment_entry_point.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::{Point, Rect, Vector2},
        runtime::GpuSurfaceCapabilities,
    };
    use std::sync::Arc;

    #[test]
    fn descriptor_selection_ignores_non_custom_shader_surfaces() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::SignalBands {
                frames: 0,
                band_count: 1,
                frame_range: [0.0, 0.0],
                samples: Arc::from([]),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        assert!(supported_custom_shader_descriptor(&surface, &mut stats).is_none());
        assert_eq!(stats.custom_shader.unsupported.surfaces, 0);
    }

    #[test]
    fn descriptor_selection_records_unsupported_custom_shader_payloads() {
        let mut stats = GpuSurfaceRenderStats::default();
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(
                    GpuShaderSurfaceDescriptor::new("test/custom-shader").uniform_bytes([1, 2, 3]),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };

        assert!(supported_custom_shader_descriptor(&surface, &mut stats).is_none());
        assert_eq!(stats.custom_shader.unsupported.surfaces, 1);
        assert_eq!(stats.custom_shader.unsupported.uniform_bytes, 3);
    }

    fn test_rect() -> Rect {
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0))
    }
}
