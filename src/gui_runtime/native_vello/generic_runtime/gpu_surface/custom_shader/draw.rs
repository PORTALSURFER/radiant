use super::super::GpuSurfaceRenderTarget;
use super::super::encoding::uniforms_as_bytes;
use super::super::gpu_surface_types::{
    CustomShaderBinding, CustomShaderPipeline, CustomShaderStaticPayloadKey, GpuSurfaceUniforms,
};
use super::super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::super::stats::GpuSurfaceRenderStats;
use super::super::upload_plan::{
    GpuSurfaceRenderCanvasUploadClass, GpuSurfaceRenderCanvasUploadCustomPresentationSource,
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
};
use super::super::visibility::visible_surface_regions;
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, PaintGpuSurface,
};
use std::time::Instant;
use vello::wgpu;

pub(super) struct CustomShaderBufferUploadRequest<'a, 'target> {
    pub(super) target: &'a mut GpuSurfaceRenderTarget<'target>,
    pub(super) surface_index: usize,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) descriptor: &'a GpuShaderSurfaceDescriptor,
    pub(super) binding: &'a mut CustomShaderBinding,
    pub(super) presentation_update: Option<&'a GpuShaderPresentationUniformUpdate>,
    pub(super) presentation_updates: &'a [GpuShaderPresentationUniformUpdate],
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
    let surface_uniform_bytes = uniforms_as_bytes(&uniforms);
    stats.record_candidate_renderer_parameter(surface_uniform_bytes.len());
    request.target.queue.write_buffer(
        &request.binding.surface_uniform_buffer,
        0,
        surface_uniform_bytes,
    );
    record_custom_shader_renderer_parameter(stats, surface_uniform_bytes.len());
    if request
        .binding
        .write_state
        .static_payload_needs_write(static_payload)
    {
        if let Some(buffer) = &request.binding.app_uniform_buffer {
            let uniform_bytes = request.descriptor.uniform_bytes.as_ref();
            stats.record_candidate_immutable_payload(uniform_bytes.len());
            request.target.queue.write_buffer(buffer, 0, uniform_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += uniform_bytes.len();
            record_custom_shader_uniform_upload(stats, uniform_bytes.len());
        }
        if let Some(buffer) = &request.binding.storage_buffer {
            let storage_bytes = request.descriptor.storage_bytes.as_ref();
            stats.record_candidate_immutable_payload(storage_bytes.len());
            request.target.queue.write_buffer(buffer, 0, storage_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += storage_bytes.len();
            record_custom_shader_storage_upload(stats, storage_bytes.len());
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
        {
            if let Some(bytes) = request.descriptor.presentation_uniform_bytes.as_deref() {
                if !presentation_uniform_write_is_available(
                    request.presentation_staging_belt.as_deref(),
                    bytes,
                ) {
                    stats.mark_candidate_unavailable(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                } else {
                    stats.record_candidate_volatile_payload(bytes.len());
                    if write_presentation_uniform(
                        request.presentation_staging_belt.as_deref_mut(),
                        request.target.encoder,
                        buffer,
                        bytes,
                    ) {
                        request.binding.write_state.cache_presentation_revision(
                            static_payload,
                            request
                                .descriptor
                                .presentation_uniform_revision
                                .unwrap_or_default(),
                        );
                        stats.custom_shader.presentation_writes += 1;
                        stats.custom_shader.presentation_write_bytes += bytes.len();
                        record_custom_shader_initial_presentation_upload(stats, bytes.len());
                    } else {
                        stats.mark_candidate_unavailable(
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        );
                    }
                }
            } else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            }
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
        {
            if !presentation_uniform_write_is_available(
                request.presentation_staging_belt.as_deref(),
                update.bytes(),
            ) {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            } else {
                stats.record_candidate_volatile_payload(update.byte_len());
                if write_presentation_uniform(
                    request.presentation_staging_belt.as_deref_mut(),
                    request.target.encoder,
                    buffer,
                    update.bytes(),
                ) {
                    request
                        .binding
                        .write_state
                        .cache_presentation_revision(static_payload, update.presentation_revision);
                    stats.custom_shader.presentation_writes += 1;
                    stats.custom_shader.presentation_write_bytes += update.byte_len();
                    record_custom_shader_update_presentation_upload(stats, update.byte_len());
                } else {
                    stats.mark_candidate_unavailable(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                }
            }
        }
    }
}

pub(super) fn upload_custom_shader_buffers_with_plan(
    mut request: CustomShaderBufferUploadRequest<'_, '_>,
    plan: &mut GpuSurfaceRenderCanvasUploadPlan,
    stats: &mut GpuSurfaceRenderStats,
) -> bool {
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
    let surface_uniform_bytes = uniforms_as_bytes(&uniforms);
    let Some(surface_uniform_byte_len) = plan.consume_upload(
        request.surface_index,
        GpuSurfaceRenderCanvasUploadClass::RendererParameter,
    ) else {
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    };
    if surface_uniform_byte_len != surface_uniform_bytes.len() {
        plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    }
    stats.record_candidate_renderer_parameter(surface_uniform_bytes.len());
    plan.mark_execution_mutated();
    request.target.queue.write_buffer(
        &request.binding.surface_uniform_buffer,
        0,
        surface_uniform_bytes,
    );
    record_custom_shader_renderer_parameter(stats, surface_uniform_bytes.len());

    let Some(static_state) =
        plan.consume_custom_static_state(request.surface_index, request.surface.key)
    else {
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    };
    if static_state.payload != static_payload
        || static_state.write
            != request
                .binding
                .write_state
                .static_payload_needs_write(static_payload)
    {
        plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    }
    if static_state.write {
        if (!request.descriptor.uniform_bytes.is_empty()
            && request.binding.app_uniform_buffer.is_none())
            || (!request.descriptor.storage_bytes.is_empty()
                && request.binding.storage_buffer.is_none())
        {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        }
        if let Some(buffer) = &request.binding.app_uniform_buffer {
            let uniform_bytes = request.descriptor.uniform_bytes.as_ref();
            let Some(byte_len) = plan.consume_upload(
                request.surface_index,
                GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            ) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return false;
            };
            if byte_len != uniform_bytes.len() {
                plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return false;
            }
            stats.record_candidate_immutable_payload(uniform_bytes.len());
            request.target.queue.write_buffer(buffer, 0, uniform_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += uniform_bytes.len();
            record_custom_shader_uniform_upload(stats, uniform_bytes.len());
        }
        if let Some(buffer) = &request.binding.storage_buffer {
            let storage_bytes = request.descriptor.storage_bytes.as_ref();
            let Some(byte_len) = plan.consume_upload(
                request.surface_index,
                GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
            ) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return false;
            };
            if byte_len != storage_bytes.len() {
                plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return false;
            }
            stats.record_candidate_immutable_payload(storage_bytes.len());
            request.target.queue.write_buffer(buffer, 0, storage_bytes);
            stats.custom_shader.static_writes += 1;
            stats.custom_shader.static_write_bytes += storage_bytes.len();
            record_custom_shader_storage_upload(stats, storage_bytes.len());
        }
        request
            .binding
            .write_state
            .cache_static_payload(static_payload);
    }

    let Some(presentation_buffer) = request.binding.presentation_uniform_buffer.as_ref() else {
        return true;
    };
    let Some(initial_state) =
        plan.consume_custom_presentation_state(request.surface_index, request.surface.key)
    else {
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    };
    let presentation_bytes = request
        .descriptor
        .presentation_uniform_bytes
        .as_deref()
        .filter(|bytes| !bytes.is_empty());
    let Some(presentation_bytes) = presentation_bytes else {
        plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    };
    let initial_revision = request
        .descriptor
        .presentation_uniform_revision
        .unwrap_or_default();
    if initial_state.payload != static_payload
        || initial_state.source != GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial
        || initial_state.revision != initial_revision
        || initial_state.byte_len != presentation_bytes.len()
        || initial_state.write
            != request
                .binding
                .write_state
                .should_upload_initial_presentation(static_payload, initial_revision)
    {
        plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    }
    if initial_state.write {
        let Some(byte_len) = plan.consume_upload(
            request.surface_index,
            GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
        ) else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        };
        if byte_len != presentation_bytes.len()
            || !presentation_uniform_write_is_available(
                request.presentation_staging_belt.as_deref(),
                presentation_bytes,
            )
        {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        }
        stats.record_candidate_volatile_payload(presentation_bytes.len());
        if !write_presentation_uniform(
            request.presentation_staging_belt.as_deref_mut(),
            request.target.encoder,
            presentation_buffer,
            presentation_bytes,
        ) {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        }
        request
            .binding
            .write_state
            .cache_presentation_revision(static_payload, initial_revision);
        stats.custom_shader.presentation_writes += 1;
        stats.custom_shader.presentation_write_bytes += presentation_bytes.len();
        record_custom_shader_initial_presentation_upload(stats, presentation_bytes.len());
    }

    let Some(update_state) =
        plan.consume_custom_presentation_state(request.surface_index, request.surface.key)
    else {
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    };
    let matching_update = super::matching_presentation_update(
        request.surface,
        request.descriptor,
        request.presentation_updates,
    );
    let (update_revision, update_byte_len, update_write) =
        matching_update.map_or((0, 0, false), |update| {
            (
                update.presentation_revision,
                update.byte_len(),
                request
                    .binding
                    .write_state
                    .presentation_update_is_acceptable(
                        static_payload,
                        update.presentation_revision,
                        presentation_bytes.len(),
                        update.byte_len(),
                    ),
            )
        });
    if update_state.payload != static_payload
        || update_state.source != GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update
        || update_state.revision != update_revision
        || update_state.byte_len != update_byte_len
        || update_state.write != update_write
    {
        plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        stats.mark_candidate_unavailable(
            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
        );
        return false;
    }
    if update_state.write {
        let Some(byte_len) = plan.consume_upload(
            request.surface_index,
            GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
        ) else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        };
        let Some(update) = matching_update else {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        };
        if byte_len != update.byte_len()
            || !presentation_uniform_write_is_available(
                request.presentation_staging_belt.as_deref(),
                update.bytes(),
            )
        {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        }
        stats.record_candidate_volatile_payload(update.byte_len());
        if !write_presentation_uniform(
            request.presentation_staging_belt.as_deref_mut(),
            request.target.encoder,
            presentation_buffer,
            update.bytes(),
        ) {
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return false;
        }
        request
            .binding
            .write_state
            .cache_presentation_revision(static_payload, update.presentation_revision);
        stats.custom_shader.presentation_writes += 1;
        stats.custom_shader.presentation_write_bytes += update.byte_len();
        record_custom_shader_update_presentation_upload(stats, update.byte_len());
    }
    true
}

fn presentation_uniform_write_is_available(
    staging_belt: Option<&wgpu::util::StagingBelt>,
    bytes: &[u8],
) -> bool {
    staging_belt.is_some() && wgpu::BufferSize::new(bytes.len() as wgpu::BufferAddress).is_some()
}

fn write_presentation_uniform(
    staging_belt: Option<&mut wgpu::util::StagingBelt>,
    encoder: &mut wgpu::CommandEncoder,
    buffer: &wgpu::Buffer,
    bytes: &[u8],
) -> bool {
    let Some(staging_belt) = staging_belt else {
        return false;
    };
    let Some(size) = wgpu::BufferSize::new(bytes.len() as wgpu::BufferAddress) else {
        return false;
    };
    let mut staging = staging_belt.write_buffer(encoder, buffer, 0, size);
    staging.copy_from_slice(bytes);
    true
}

fn record_custom_shader_renderer_parameter(stats: &mut GpuSurfaceRenderStats, byte_len: usize) {
    stats
        .render_canvas_uploads
        .record_renderer_parameter(byte_len);
}

fn record_custom_shader_uniform_upload(stats: &mut GpuSurfaceRenderStats, byte_len: usize) {
    stats
        .render_canvas_uploads
        .record_immutable_payload(byte_len);
}

fn record_custom_shader_storage_upload(stats: &mut GpuSurfaceRenderStats, byte_len: usize) {
    stats
        .render_canvas_uploads
        .record_immutable_payload(byte_len);
}

fn record_custom_shader_initial_presentation_upload(
    stats: &mut GpuSurfaceRenderStats,
    byte_len: usize,
) {
    stats
        .render_canvas_uploads
        .record_volatile_payload(byte_len);
}

fn record_custom_shader_update_presentation_upload(
    stats: &mut GpuSurfaceRenderStats,
    byte_len: usize,
) {
    stats
        .render_canvas_uploads
        .record_volatile_payload(byte_len);
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
