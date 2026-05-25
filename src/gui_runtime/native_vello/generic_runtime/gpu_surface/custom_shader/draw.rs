use super::super::GpuSurfaceRenderTarget;
use super::super::encoding::uniforms_as_bytes;
use super::super::gpu_surface_types::{
    CustomShaderBinding, CustomShaderPipeline, GpuSurfaceUniforms,
};
use super::super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::visibility::visible_surface_regions;
use crate::gui::types::Rect as UiRect;
use crate::runtime::{GpuShaderSurfaceDescriptor, PaintGpuSurface};
use std::time::Instant;

pub(super) struct CustomShaderBufferUploadRequest<'a, 'target> {
    pub(super) target: &'a mut GpuSurfaceRenderTarget<'target>,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) descriptor: &'a GpuShaderSurfaceDescriptor,
    pub(super) binding: &'a CustomShaderBinding,
}

pub(super) struct CustomShaderDrawRequest<'a, 'target> {
    pub(super) target: &'a mut GpuSurfaceRenderTarget<'target>,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) descriptor: &'a GpuShaderSurfaceDescriptor,
    pub(super) pipeline: &'a CustomShaderPipeline,
    pub(super) binding: &'a CustomShaderBinding,
    pub(super) occlusion_regions: &'a [UiRect],
}

pub(super) fn upload_custom_shader_buffers(request: CustomShaderBufferUploadRequest<'_, '_>) {
    let uniforms = GpuSurfaceUniforms {
        dest: surface_dest(request.surface, request.target.dpi_scale),
        target_size: [
            request.target.size.x.max(1.0),
            request.target.size.y.max(1.0),
        ],
        ..GpuSurfaceUniforms::default()
    };
    request.target.queue.write_buffer(
        &request.binding.surface_uniform_buffer,
        0,
        uniforms_as_bytes(&uniforms),
    );
    if let Some(buffer) = &request.binding.app_uniform_buffer {
        request
            .target
            .queue
            .write_buffer(buffer, 0, &request.descriptor.uniform_bytes);
    }
    if let Some(buffer) = &request.binding.storage_buffer {
        request
            .target
            .queue
            .write_buffer(buffer, 0, &request.descriptor.storage_bytes);
    }
}

pub(super) fn encode_custom_shader_draw(
    request: CustomShaderDrawRequest<'_, '_>,
    stats: &mut GpuSurfaceRenderStats,
) {
    let started = Instant::now();
    let mut pass = gpu_surface_render_pass(request.target.encoder, request.target.target_view);
    pass.set_pipeline(&request.pipeline.pipeline);
    pass.set_bind_group(0, &request.binding.bind_group, &[]);
    for region in visible_surface_regions(request.surface.rect, request.occlusion_regions) {
        if set_surface_scissor(&mut pass, region, request.target.dpi_scale) {
            pass.draw(0..request.descriptor.vertex_count, 0..1);
        }
    }
    drop(pass);
    stats.composite.encode_elapsed += started.elapsed();
}
