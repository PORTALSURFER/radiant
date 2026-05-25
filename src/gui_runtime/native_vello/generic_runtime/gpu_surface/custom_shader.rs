use super::encoding::uniforms_as_bytes;
use super::gpu_surface_types::{CustomShaderBinding, CustomShaderPipeline, GpuSurfaceUniforms};
use super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::stats::GpuSurfaceRenderStats;
use super::visibility::visible_surface_regions;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{GpuShaderSurfaceDescriptor, GpuSurfaceContent, PaintGpuSurface};
use std::time::Instant;
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/pipeline.rs"]
mod pipeline;
use diagnostics::{record_failed_custom_shader_surface, record_unsupported_custom_shader};
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
        upload_custom_shader_buffers(target, surface, descriptor, binding);
        encode_custom_shader_draw(
            target,
            surface,
            descriptor,
            pipeline,
            binding,
            occlusion_regions,
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
        self.ensure_custom_shader_binding(target.device, surface.key, descriptor, stats);
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
    if descriptor.wgsl_source.is_none() || descriptor.fragment_entry_point.is_none() {
        record_unsupported_custom_shader(descriptor, stats);
        return None;
    }
    Some(descriptor)
}

fn upload_custom_shader_buffers(
    target: &mut GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    descriptor: &GpuShaderSurfaceDescriptor,
    binding: &CustomShaderBinding,
) {
    let uniforms = GpuSurfaceUniforms {
        dest: surface_dest(surface, target.dpi_scale),
        target_size: [target.size.x.max(1.0), target.size.y.max(1.0)],
        ..GpuSurfaceUniforms::default()
    };
    target.queue.write_buffer(
        &binding.surface_uniform_buffer,
        0,
        uniforms_as_bytes(&uniforms),
    );
    if let Some(buffer) = &binding.app_uniform_buffer {
        target
            .queue
            .write_buffer(buffer, 0, &descriptor.uniform_bytes);
    }
    if let Some(buffer) = &binding.storage_buffer {
        target
            .queue
            .write_buffer(buffer, 0, &descriptor.storage_bytes);
    }
}

fn encode_custom_shader_draw(
    target: &mut GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    descriptor: &GpuShaderSurfaceDescriptor,
    pipeline: &CustomShaderPipeline,
    binding: &CustomShaderBinding,
    occlusion_regions: &[UiRect],
    stats: &mut GpuSurfaceRenderStats,
) {
    let started = Instant::now();
    let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
    pass.set_pipeline(&pipeline.pipeline);
    pass.set_bind_group(0, &binding.bind_group, &[]);
    for region in visible_surface_regions(surface.rect, occlusion_regions) {
        if set_surface_scissor(&mut pass, region, target.dpi_scale) {
            pass.draw(0..descriptor.vertex_count, 0..1);
        }
    }
    drop(pass);
    stats.composite.encode_elapsed += started.elapsed();
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
