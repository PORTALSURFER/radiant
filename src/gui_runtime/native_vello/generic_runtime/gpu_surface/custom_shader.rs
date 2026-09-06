use super::gpu_surface_types::{
    CustomShaderBindingKey, CustomShaderBindingWriteState, CustomShaderPipelineKey,
    CustomShaderStaticPayloadKey,
};
use super::stats::GpuSurfaceRenderStats;
use super::upload_plan::{
    GpuSurfaceRenderCanvasUploadAction, GpuSurfaceRenderCanvasUploadClass,
    GpuSurfaceRenderCanvasUploadCustomPresentationSource, GpuSurfaceRenderCanvasUploadPlan,
    GpuSurfaceRenderCanvasUploadPlanUnavailableReason, GpuSurfaceRenderCanvasUploadSurface,
};
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer, PRESENTATION_STAGING_BELT_CHUNK_SIZE};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuShaderPresentationUniformUpdate, GpuShaderSurfaceDescriptor, GpuSurfaceContent,
    PaintGpuSurface,
};
use std::collections::HashMap;
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/draw.rs"]
mod draw;
#[path = "custom_shader/pipeline.rs"]
mod pipeline;
use binding::CustomShaderBindingRequest;
use diagnostics::record_failed_custom_shader_surface;
use draw::{CustomShaderBufferUploadRequest, CustomShaderDrawRequest};
use pipeline::{CustomShaderPipelineRequest, custom_shader_pipeline_key};

const MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS: usize = 1024;

pub(super) fn record_unsupported_custom_shader(
    descriptor: &GpuShaderSurfaceDescriptor,
    stats: &mut GpuSurfaceRenderStats,
) {
    diagnostics::record_unsupported_custom_shader(descriptor, stats);
}

pub(super) struct CustomShaderRenderRequest<'a> {
    pub(super) surface_index: usize,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) occlusion_regions: &'a [UiRect],
    pub(super) presentation_updates: &'a [GpuShaderPresentationUniformUpdate],
}

#[derive(Clone)]
struct CustomShaderPipelinePreflightIdentity {
    device: usize,
    format: vello::wgpu::TextureFormat,
    key: CustomShaderPipelineKey,
}

#[derive(Clone)]
struct CustomShaderBindingPreflightIdentity {
    cache_key: CustomShaderBindingKey,
    write_state: CustomShaderBindingWriteState,
}

#[derive(Default)]
pub(super) struct CustomShaderUploadPreflightState {
    pipelines: HashMap<u64, Option<CustomShaderPipelinePreflightIdentity>>,
    bindings: HashMap<u64, Option<CustomShaderBindingPreflightIdentity>>,
}

pub(super) struct CustomShaderUploadPreflight {
    pub(super) renderable: bool,
    pub(super) unavailable: Option<GpuSurfaceRenderCanvasUploadPlanUnavailableReason>,
}

impl CustomShaderUploadPreflightState {
    pub(super) fn reset(&mut self) {
        self.pipelines.clear();
        self.bindings.clear();
        self.pipelines
            .shrink_to(MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS);
        self.bindings
            .shrink_to(MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS);
    }

    #[cfg(test)]
    pub(super) fn pipelines_capacity(&self) -> usize {
        self.pipelines.capacity()
    }

    #[cfg(test)]
    pub(super) fn bindings_capacity(&self) -> usize {
        self.bindings.capacity()
    }

    fn pipeline_decision(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        surface_key: u64,
        device: usize,
        format: vello::wgpu::TextureFormat,
        key: &CustomShaderPipelineKey,
    ) -> bool {
        let cached = self
            .pipelines
            .get(&surface_key)
            .cloned()
            .unwrap_or_else(|| {
                renderer
                    .resources
                    .custom_shader_pipeline_identity(surface_key)
                    .map(|pipeline| CustomShaderPipelinePreflightIdentity {
                        device: pipeline.device,
                        format: pipeline.format,
                        key: pipeline.key.clone(),
                    })
            });
        let rebuild = cached.as_ref().is_none_or(|cached| {
            cached.device != device || cached.format != format || cached.key != *key
        });
        self.pipelines.insert(
            surface_key,
            Some(CustomShaderPipelinePreflightIdentity {
                device,
                format,
                key: key.clone(),
            }),
        );
        if rebuild {
            self.bindings.insert(surface_key, None);
        }
        rebuild
    }

    fn can_track_pipeline(&self, surface_key: u64) -> bool {
        self.pipelines.contains_key(&surface_key)
            || self.pipelines.len() < MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS
    }

    fn binding_decision(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        surface_key: u64,
        cache_key: &CustomShaderBindingKey,
    ) -> (bool, CustomShaderBindingWriteState) {
        let cached = self.bindings.get(&surface_key).cloned().unwrap_or_else(|| {
            renderer
                .resources
                .custom_shader_bindings
                .get(&surface_key)
                .map(|binding| CustomShaderBindingPreflightIdentity {
                    cache_key: binding.cache_key.clone(),
                    write_state: binding.write_state,
                })
        });
        let rebuild = cached
            .as_ref()
            .is_none_or(|cached| cached.cache_key != *cache_key);
        let write_state = if rebuild {
            CustomShaderBindingWriteState::default()
        } else {
            cached
                .as_ref()
                .map_or_else(CustomShaderBindingWriteState::default, |cached| {
                    cached.write_state
                })
        };
        self.bindings.insert(
            surface_key,
            Some(CustomShaderBindingPreflightIdentity {
                cache_key: cache_key.clone(),
                write_state,
            }),
        );
        (rebuild, write_state)
    }

    fn cache_static_payload(&mut self, surface_key: u64, payload: CustomShaderStaticPayloadKey) {
        if let Some(Some(binding)) = self.bindings.get_mut(&surface_key) {
            binding.write_state.cache_static_payload(payload);
        }
    }

    fn cache_presentation_revision(
        &mut self,
        surface_key: u64,
        payload: CustomShaderStaticPayloadKey,
        revision: u64,
    ) {
        if let Some(Some(binding)) = self.bindings.get_mut(&surface_key) {
            binding
                .write_state
                .cache_presentation_revision(payload, revision);
        }
    }
}

impl GpuSurfaceRenderer {
    pub(super) fn preflight_custom_shader_upload_actions(
        &self,
        target: super::upload_plan::GpuSurfaceRenderCanvasUploadTarget,
        surface_index: usize,
        surface: &PaintGpuSurface,
        presentation_updates: &[GpuShaderPresentationUniformUpdate],
        state: &mut CustomShaderUploadPreflightState,
        actions: &mut Vec<GpuSurfaceRenderCanvasUploadAction>,
    ) -> CustomShaderUploadPreflight {
        let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index,
                key: surface.key,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid,
            });
            return CustomShaderUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
            };
        };
        if !custom_shader_descriptor_is_supported(descriptor) || !surface.content.is_renderable() {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index,
                key: surface.key,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
            });
            return CustomShaderUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported),
            };
        }
        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index,
                key: surface.key,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
            });
            return CustomShaderUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported),
            };
        };
        if !state.can_track_pipeline(surface.key) {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index,
                key: surface.key,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            });
            return CustomShaderUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete),
            };
        }
        actions.push(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index,
            key: surface.key,
            surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        });
        let device = target.device;
        let pipeline_rebuild =
            state.pipeline_decision(self, surface.key, device, target.format, &pipeline_key);
        actions.push(GpuSurfaceRenderCanvasUploadAction::CustomPipeline {
            surface_index,
            key: surface.key,
            device,
            format: target.format,
            pipeline_key: pipeline_key.clone(),
            rebuild: pipeline_rebuild,
        });

        let binding_cache_key = binding::custom_shader_binding_key(&pipeline_key, descriptor);
        let (binding_rebuild, mut write_state) =
            state.binding_decision(self, surface.key, &binding_cache_key);
        actions.push(GpuSurfaceRenderCanvasUploadAction::CustomBinding {
            surface_index,
            key: surface.key,
            cache_key: binding_cache_key,
            rebuild: binding_rebuild,
        });

        let static_payload = CustomShaderStaticPayloadKey::new(
            descriptor.storage_identity,
            descriptor.storage_revision,
            descriptor.uniform_bytes.len(),
            descriptor.storage_bytes.len(),
        );
        actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: std::mem::size_of::<super::gpu_surface_types::GpuSurfaceUniforms>(),
        });
        let static_write = write_state.static_payload_needs_write(static_payload);
        actions.push(GpuSurfaceRenderCanvasUploadAction::CustomStaticState {
            surface_index,
            key: surface.key,
            payload: static_payload,
            write: static_write,
        });
        if static_write {
            if !descriptor.uniform_bytes.is_empty() {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                    byte_len: descriptor.uniform_bytes.len(),
                });
            }
            if !descriptor.storage_bytes.is_empty() {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                    byte_len: descriptor.storage_bytes.len(),
                });
            }
            write_state.cache_static_payload(static_payload);
            state.cache_static_payload(surface.key, static_payload);
        }

        if let Some(presentation_bytes) = descriptor
            .presentation_uniform_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
        {
            let initial_revision = descriptor.presentation_uniform_revision.unwrap_or_default();
            let initial_write =
                write_state.should_upload_initial_presentation(static_payload, initial_revision);
            actions.push(
                GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                    surface_index,
                    key: surface.key,
                    payload: static_payload,
                    revision: initial_revision,
                    byte_len: presentation_bytes.len(),
                    source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial,
                    write: initial_write,
                },
            );
            if initial_write {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
                    byte_len: presentation_bytes.len(),
                });
                write_state.cache_presentation_revision(static_payload, initial_revision);
                state.cache_presentation_revision(surface.key, static_payload, initial_revision);
            }

            let matching_update =
                matching_presentation_update(surface, descriptor, presentation_updates);
            let (update_revision, update_byte_len, update_write) =
                matching_update.map_or((0, 0, false), |update| {
                    (
                        update.presentation_revision,
                        update.byte_len(),
                        write_state.presentation_update_is_acceptable(
                            static_payload,
                            update.presentation_revision,
                            presentation_bytes.len(),
                            update.byte_len(),
                        ),
                    )
                });
            actions.push(
                GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                    surface_index,
                    key: surface.key,
                    payload: static_payload,
                    revision: update_revision,
                    byte_len: update_byte_len,
                    source: GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update,
                    write: update_write,
                },
            );
            if update_write {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
                    byte_len: update_byte_len,
                });
                state.cache_presentation_revision(surface.key, static_payload, update_revision);
            }
        }

        CustomShaderUploadPreflight {
            renderable: true,
            unavailable: None,
        }
    }
}

impl GpuSurfaceRenderer {
    fn render_custom_shader_legacy(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: &CustomShaderRenderRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        let surface = request.surface;
        let occlusion_regions = request.occlusion_regions;
        let presentation_updates = request.presentation_updates;
        let Some(descriptor) = supported_custom_shader_descriptor(surface, stats) else {
            return false;
        };
        if !self.prepare_custom_shader_resources(target, surface, descriptor, stats) {
            return false;
        }
        let presentation_update =
            matching_presentation_update(surface, descriptor, presentation_updates);
        let presentation_staging_belt = descriptor
            .presentation_uniform_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|_| {
                self.presentation_staging_belt.get_or_insert_with(|| {
                    vello::wgpu::util::StagingBelt::new(
                        target.device.clone(),
                        PRESENTATION_STAGING_BELT_CHUNK_SIZE,
                    )
                })
            });
        {
            let Some(binding) = self.resources.custom_shader_bindings.get_mut(&surface.key) else {
                record_failed_custom_shader_surface(stats);
                return false;
            };
            draw::upload_custom_shader_buffers(
                CustomShaderBufferUploadRequest {
                    target,
                    surface_index: 0,
                    surface,
                    descriptor,
                    binding,
                    presentation_update,
                    presentation_updates,
                    presentation_staging_belt,
                },
                stats,
            );
        }
        let Some(pipeline) = self.resources.custom_shader_pipeline(surface.key) else {
            record_failed_custom_shader_surface(stats);
            return false;
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return false;
        };
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
        true
    }

    pub(super) fn render_custom_shader(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: CustomShaderRenderRequest<'_>,
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        if let Some(upload_plan) = upload_plan {
            if upload_plan.execution_is_available()
                && let Some(rendered) =
                    self.render_custom_shader_with_plan(target, &request, upload_plan, stats)
            {
                return rendered;
            }
            if let Some(aborted) = custom_shader_plan_failure_result(upload_plan) {
                return aborted;
            }
        }
        self.render_custom_shader_legacy(target, &request, stats)
    }

    fn render_custom_shader_with_plan(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: &CustomShaderRenderRequest<'_>,
        upload_plan: &mut GpuSurfaceRenderCanvasUploadPlan,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<bool> {
        let surface_index = request.surface_index;
        let surface = request.surface;
        let occlusion_regions = request.occlusion_regions;
        let presentation_updates = request.presentation_updates;
        match upload_plan.consume_surface_decision(surface_index, surface.key) {
            Some(Ok(decision))
                if decision.surface == GpuSurfaceRenderCanvasUploadSurface::CustomShader => {}
            Some(Err(reason)) => {
                stats.mark_candidate_unavailable(reason);
                if reason == GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported
                    && let GpuSurfaceContent::CustomShader { descriptor } = &surface.content
                {
                    record_unsupported_custom_shader(descriptor, stats);
                }
                return Some(false);
            }
            Some(Ok(_)) | None => {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return None;
            }
        }
        let Some(descriptor) = supported_custom_shader_descriptor(surface, stats) else {
            return Some(false);
        };
        if !surface.content.is_renderable() {
            record_unsupported_custom_shader(descriptor, stats);
            return Some(false);
        }
        let Some(pipeline_execution) =
            upload_plan.consume_custom_pipeline(surface_index, surface.key)
        else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return None;
        };
        let Some(pipeline_key) = custom_shader_pipeline_key(descriptor) else {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported);
            record_unsupported_custom_shader(descriptor, stats);
            return Some(false);
        };
        let pipeline_request = CustomShaderPipelineRequest {
            surface_key: surface.key,
            device: target.device,
            target_format: target.format,
            key: pipeline_key.clone(),
        };
        if pipeline_execution.surface_index != surface_index
            || pipeline_execution.key != surface.key
            || pipeline_execution.device != super::wgpu_device_id(target.device)
            || pipeline_execution.format != target.format
            || pipeline_execution.pipeline_key != pipeline_key
            || pipeline_execution.rebuild
                != self.custom_shader_pipeline_needs_rebuild(&pipeline_request)
        {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return None;
        }
        if pipeline_execution.rebuild {
            upload_plan.mark_execution_mutated();
        }
        self.ensure_custom_shader_pipeline(pipeline_request, stats);
        if !self.resources.has_custom_shader_pipeline(surface.key) {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            record_failed_custom_shader_surface(stats);
            return Some(false);
        }

        let Some(binding_execution) =
            upload_plan.consume_custom_binding(surface_index, surface.key)
        else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return None;
        };
        let expected_binding_key = binding::custom_shader_binding_key(&pipeline_key, descriptor);
        if binding_execution.surface_index != surface_index
            || binding_execution.key != surface.key
            || binding_execution.cache_key != expected_binding_key
            || binding_execution.rebuild
                != self.custom_shader_binding_needs_rebuild(surface.key, &expected_binding_key)
        {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return None;
        }
        if binding_execution.rebuild {
            upload_plan.mark_execution_mutated();
        }
        self.ensure_custom_shader_binding(
            CustomShaderBindingRequest {
                device: target.device,
                surface_key: surface.key,
                descriptor,
            },
            stats,
        );
        if !self
            .resources
            .custom_shader_bindings
            .contains_key(&surface.key)
        {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            record_failed_custom_shader_surface(stats);
            return Some(false);
        }

        let presentation_update =
            matching_presentation_update(surface, descriptor, presentation_updates);
        if descriptor
            .presentation_uniform_bytes
            .as_ref()
            .is_some_and(|bytes| !bytes.is_empty())
            && self.presentation_staging_belt.is_none()
        {
            upload_plan.mark_execution_mutated();
        }
        let presentation_staging_belt = descriptor
            .presentation_uniform_bytes
            .as_ref()
            .filter(|bytes| !bytes.is_empty())
            .map(|_| {
                self.presentation_staging_belt.get_or_insert_with(|| {
                    vello::wgpu::util::StagingBelt::new(
                        target.device.clone(),
                        PRESENTATION_STAGING_BELT_CHUNK_SIZE,
                    )
                })
            });
        let upload_succeeded = {
            let Some(binding) = self.resources.custom_shader_bindings.get_mut(&surface.key) else {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                record_failed_custom_shader_surface(stats);
                return Some(false);
            };
            draw::upload_custom_shader_buffers_with_plan(
                CustomShaderBufferUploadRequest {
                    target,
                    surface_index,
                    surface,
                    descriptor,
                    binding,
                    presentation_update,
                    presentation_updates,
                    presentation_staging_belt,
                },
                upload_plan,
                stats,
            )
        };
        if !upload_succeeded {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            return custom_shader_plan_failure_result(upload_plan);
        }

        let Some(pipeline) = self.resources.custom_shader_pipeline(surface.key) else {
            record_failed_custom_shader_surface(stats);
            return Some(false);
        };
        let Some(binding) = self.resources.custom_shader_bindings.get(&surface.key) else {
            record_failed_custom_shader_surface(stats);
            return Some(false);
        };
        if !upload_plan.consume_action(GpuSurfaceRenderCanvasUploadAction::Activate {
            surface_index,
            key: surface.key,
        }) {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return custom_shader_plan_failure_result(upload_plan);
        }
        upload_plan.mark_execution_mutated();
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
        Some(true)
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
        if !self.resources.has_custom_shader_pipeline(surface.key) {
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

pub(super) fn custom_shader_plan_failure_result(
    upload_plan: &GpuSurfaceRenderCanvasUploadPlan,
) -> Option<bool> {
    upload_plan.execution_mutated().then_some(false)
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

fn matching_presentation_update<'a>(
    surface: &PaintGpuSurface,
    descriptor: &GpuShaderSurfaceDescriptor,
    updates: &'a [GpuShaderPresentationUniformUpdate],
) -> Option<&'a GpuShaderPresentationUniformUpdate> {
    let presentation_byte_len = descriptor
        .presentation_uniform_bytes
        .as_ref()
        .map_or(0, |bytes| bytes.len());
    if presentation_byte_len == 0 {
        return None;
    }
    updates
        .iter()
        .filter(|update| {
            update.widget_id == surface.widget_id
                && update.surface_key.get() == surface.key
                && update.storage_identity == descriptor.storage_identity
                && update.storage_revision == descriptor.storage_revision
                && update.byte_len() == presentation_byte_len
        })
        .max_by_key(|update| update.presentation_revision)
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

    #[test]
    fn presentation_updates_match_surface_storage_and_byte_shape() {
        let descriptor = GpuShaderSurfaceDescriptor::new("test/custom-shader")
            .storage_identity(11)
            .storage_revision(13)
            .presentation_uniform([1, 2, 3, 4], 2);
        let surface = PaintGpuSurface {
            widget_id: 17,
            key: 93,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(descriptor.clone()),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        };
        let matching = GpuShaderPresentationUniformUpdate::try_new(17, 93, 11, 13, 4, [4, 5, 6, 7])
            .expect("valid matching presentation update");
        let wrong_storage =
            GpuShaderPresentationUniformUpdate::try_new(17, 93, 12, 13, 5, [7, 8, 9, 10])
                .expect("valid mismatched presentation update");
        let wrong_length = GpuShaderPresentationUniformUpdate::try_new(17, 93, 11, 13, 6, [10; 8])
            .expect("valid mismatched presentation update");

        assert_eq!(
            matching_presentation_update(&surface, &descriptor, &[wrong_storage, matching]),
            Some(&matching)
        );
        assert!(matching_presentation_update(&surface, &descriptor, &[wrong_length]).is_none());
    }

    fn test_rect() -> Rect {
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0))
    }
}
