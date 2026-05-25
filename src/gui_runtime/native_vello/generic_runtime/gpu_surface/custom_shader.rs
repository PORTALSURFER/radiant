use super::encoding::uniforms_as_bytes;
use super::gpu_surface_types::GpuSurfaceUniforms;
use super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::stats::GpuSurfaceRenderStats;
use super::visibility::visible_surface_regions;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{GpuSurfaceContent, PaintGpuSurface};
use std::time::Instant;
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/pipeline.rs"]
mod pipeline;
use diagnostics::{record_failed_custom_shader_surface, record_unsupported_custom_shader};
use pipeline::custom_shader_pipeline_key;

impl GpuSurfaceRenderer {
    pub(super) fn render_custom_shader(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
            return;
        };
        if descriptor.wgsl_source.is_none() || descriptor.fragment_entry_point.is_none() {
            record_unsupported_custom_shader(descriptor, stats);
            return;
        }

        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            record_unsupported_custom_shader(descriptor, stats);
            return;
        };
        self.ensure_custom_shader_pipeline(
            surface.key,
            target.device,
            target.format,
            pipeline_key,
            stats,
        );
        if !self
            .resources
            .custom_shader_pipelines
            .contains_key(&surface.key)
        {
            record_failed_custom_shader_surface(stats);
            return;
        }
        self.ensure_custom_shader_binding(target.device, surface.key, descriptor, stats);
        let Some(pipeline) = self.resources.custom_shader_pipelines.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return;
        };
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface),
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
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &binding.bind_group, &[]);
        for region in visible_surface_regions(surface.rect, occlusion_regions) {
            if set_surface_scissor(&mut pass, region) {
                pass.draw(0..descriptor.vertex_count, 0..1);
            }
        }
        stats.custom_shader.surfaces_rendered += 1;
        stats.composite.encode_elapsed += started.elapsed();
    }
}
