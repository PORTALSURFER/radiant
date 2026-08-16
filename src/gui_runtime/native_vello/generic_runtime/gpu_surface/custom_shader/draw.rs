use super::super::GpuSurfaceRenderTarget;
use super::super::encoding::uniforms_as_bytes;
use super::super::gpu_surface_types::{
    CustomShaderBinding, CustomShaderPipeline, CustomShaderStaticPayloadKey, GpuSurfaceUniforms,
};
use super::super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::visibility::visible_surface_regions;
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, PaintGpuSurface,
};
use std::time::Instant;
use vello::wgpu;

pub(super) struct CustomShaderBufferUploadRequest<'a, 'target> {
    pub(super) target: &'a mut GpuSurfaceRenderTarget<'target>,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) descriptor: &'a GpuShaderSurfaceDescriptor,
    pub(super) binding: &'a mut CustomShaderBinding,
    pub(super) presentation_update: Option<&'a GpuShaderPresentationUniformUpdate>,
    pub(super) presentation_staging_belt: Option<&'a mut wgpu::util::StagingBelt>,
}

pub(super) struct CustomShaderDrawRequest<'a, 'target> {
    pub(super) target: &'a mut GpuSurfaceRenderTarget<'target>,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) descriptor: &'a GpuShaderSurfaceDescriptor,
    pub(super) pipeline: &'a CustomShaderPipeline,
    pub(super) binding: &'a CustomShaderBinding,
    pub(super) occlusion_regions: &'a [UiRect],
}

pub(super) fn upload_custom_shader_buffers(
    mut request: CustomShaderBufferUploadRequest<'_, '_>,
    stats: &mut GpuSurfaceRenderStats,
) {
    let static_payload = CustomShaderStaticPayloadKey::new(
        request.descriptor.storage_identity,
        request.descriptor.storage_revision,
        request.descriptor.uniform_bytes.len(),
        request.descriptor.storage_bytes.len(),
    );
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
    if request
        .binding
        .write_state
        .static_payload_needs_write(static_payload)
    {
        if let Some(buffer) = &request.binding.app_uniform_buffer {
            request
                .target
                .queue
                .write_buffer(buffer, 0, &request.descriptor.uniform_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += request.descriptor.uniform_bytes.len();
        }
        if let Some(buffer) = &request.binding.storage_buffer {
            request
                .target
                .queue
                .write_buffer(buffer, 0, &request.descriptor.storage_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += request.descriptor.storage_bytes.len();
        }
        request
            .binding
            .write_state
            .cache_static_payload(static_payload);
    }
    if let Some(buffer) = &request.binding.presentation_uniform_buffer {
        if request
            .binding
            .write_state
            .should_upload_initial_presentation(
                static_payload,
                request
                    .descriptor
                    .presentation_uniform_revision
                    .unwrap_or_default(),
            )
            && let Some(bytes) = request.descriptor.presentation_uniform_bytes.as_deref()
            && write_presentation_uniform(
                request.presentation_staging_belt.as_deref_mut(),
                request.target.encoder,
                buffer,
                bytes,
                request.target.device,
            )
        {
            request.binding.write_state.cache_presentation_revision(
                static_payload,
                request
                    .descriptor
                    .presentation_uniform_revision
                    .unwrap_or_default(),
            );
            stats.custom_shader.presentation_writes += 1;
            stats.custom_shader.presentation_write_bytes += bytes.len();
        }
        if let Some(update) = request.presentation_update
            && request
                .binding
                .write_state
                .presentation_update_is_acceptable(
                    static_payload,
                    update.presentation_revision,
                    request
                        .descriptor
                        .presentation_uniform_bytes
                        .as_ref()
                        .map_or(0, |bytes| bytes.len()),
                    update.byte_len(),
                )
            && write_presentation_uniform(
                request.presentation_staging_belt.as_deref_mut(),
                request.target.encoder,
                buffer,
                update.bytes(),
                request.target.device,
            )
        {
            request
                .binding
                .write_state
                .cache_presentation_revision(static_payload, update.presentation_revision);
            stats.custom_shader.presentation_writes += 1;
            stats.custom_shader.presentation_write_bytes += update.byte_len();
        }
    }
}

fn write_presentation_uniform(
    staging_belt: Option<&mut wgpu::util::StagingBelt>,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    bytes: &[u8],
    device: &wgpu::Device,
) -> bool {
    let Some(staging_belt) = staging_belt else {
        return false;
    };
    let Some(size) = wgpu::BufferSize::new(bytes.len() as wgpu::BufferAddress) else {
        return false;
    };
    let mut staging = staging_belt.write_buffer(encoder, buffer, 0, size, device);
    staging.copy_from_slice(bytes);
    true
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
