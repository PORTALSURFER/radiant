use super::gpu_surface_types::{
    CustomShaderBindingKey, CustomShaderBindingWriteState, CustomShaderPipelineIdentity,
    CustomShaderStaticPayloadKey,
};
use super::persistent_storage::{
    PersistentStorageBindingCursor, PersistentStorageSelection, select_persistent_storage,
};
use super::resources::{
    CustomShaderFrameRequest, CustomShaderPreflightCache,
    MAX_CUSTOM_SHADER_FRAME_REQUEST_KEY_BYTES, MAX_CUSTOM_SHADER_FRAME_REQUESTS,
    custom_shader_frame_requests_fit,
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
use std::collections::{HashMap, HashSet};
#[path = "custom_shader/binding.rs"]
mod binding;
#[path = "custom_shader/diagnostics.rs"]
mod diagnostics;
#[path = "custom_shader/draw.rs"]
mod draw;
#[path = "custom_shader/pipeline.rs"]
pub(in crate::gui_runtime::native_vello::generic_runtime) mod pipeline;
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

pub(super) struct CustomShaderDataUpdates<'a> {
    pub(super) presentation: &'a [GpuShaderPresentationUniformUpdate],
    pub(super) persistent: &'a crate::runtime::GpuPersistentStorageStore,
}

pub(super) struct CustomShaderRenderRequest<'a> {
    pub(super) surface_index: usize,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) occlusion_regions: &'a [UiRect],
    pub(super) persistent_storage: &'a crate::runtime::GpuPersistentStorageStore,
    pub(super) presentation_updates: &'a [GpuShaderPresentationUniformUpdate],
}

#[derive(Clone)]
struct CustomShaderBindingPreflightIdentity {
    cache_key: CustomShaderBindingKey,
    write_state: CustomShaderBindingWriteState,
}

#[derive(Default)]
pub(super) struct CustomShaderUploadPreflightState {
    cache: Option<CustomShaderPreflightCache>,
    pub(super) defer_transition: bool,
    bindings: HashMap<u64, Option<CustomShaderBindingPreflightIdentity>>,
    persistent_cursors: HashMap<u64, PersistentStorageBindingCursor>,
}

pub(super) struct CustomShaderUploadPreflight {
    pub(super) renderable: bool,
    pub(super) unavailable: Option<GpuSurfaceRenderCanvasUploadPlanUnavailableReason>,
}

impl CustomShaderUploadPreflightState {
    pub(super) fn reset(&mut self, cache: Option<CustomShaderPreflightCache>) {
        self.cache = cache;
        self.defer_transition = false;
        self.bindings.clear();
        self.persistent_cursors.clear();
        self.persistent_cursors
            .shrink_to(MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS);
        self.bindings
            .shrink_to(MAX_CUSTOM_SHADER_PREFLIGHT_ASSOCIATIONS);
    }

    #[cfg(test)]
    pub(super) fn pipelines_capacity(&self) -> usize {
        self.cache
            .as_ref()
            .map_or(0, CustomShaderPreflightCache::pipelines_capacity)
    }

    #[cfg(test)]
    pub(super) fn bindings_capacity(&self) -> usize {
        self.bindings.capacity()
    }

    fn pipeline_decision(
        &mut self,
        surface_key: u64,
        identity: &CustomShaderPipelineIdentity,
    ) -> Option<bool> {
        let rebuild = self
            .cache
            .as_mut()?
            .pipeline_decision(surface_key, identity)?;
        if rebuild {
            self.bindings.insert(surface_key, None);
        }
        Some(rebuild)
    }

    fn binding_decision(
        &mut self,
        surface_key: u64,
        cache_key: &CustomShaderBindingKey,
    ) -> (bool, CustomShaderBindingWriteState) {
        let cached = self.bindings.get(&surface_key).cloned().unwrap_or_else(|| {
            self.cache
                .as_ref()?
                .binding(surface_key)
                .map(
                    |(cache_key, write_state)| CustomShaderBindingPreflightIdentity {
                        cache_key,
                        write_state,
                    },
                )
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

pub(super) fn custom_shader_frame_requests(
    primitives: &[crate::runtime::PaintPrimitive],
    device: usize,
    format: vello::wgpu::TextureFormat,
) -> Option<Vec<CustomShaderFrameRequest>> {
    let mut requests = Vec::new();
    let mut request_key_bytes = 0usize;
    let mut canonical_identities: HashSet<CustomShaderPipelineIdentity> = HashSet::new();
    for primitive in primitives {
        let crate::runtime::PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
            continue;
        };
        if !surface.rect.has_finite_positive_area()
            || !surface.content.is_renderable()
            || !custom_shader_descriptor_is_supported(descriptor)
        {
            continue;
        }
        if requests.len() >= MAX_CUSTOM_SHADER_FRAME_REQUESTS {
            return None;
        }
        let Some(key) = custom_shader_pipeline_key(descriptor) else {
            continue;
        };
        let identity = CustomShaderPipelineIdentity {
            device,
            format,
            key,
        };
        let identity = if let Some(existing) = canonical_identities.get(&identity) {
            existing.clone()
        } else {
            request_key_bytes = request_key_bytes.checked_add(identity.key.text_bytes())?;
            if request_key_bytes > MAX_CUSTOM_SHADER_FRAME_REQUEST_KEY_BYTES {
                return None;
            }
            canonical_identities.insert(identity.clone());
            identity
        };
        requests.push(CustomShaderFrameRequest {
            surface_key: surface.key,
            binding_key: binding::custom_shader_binding_key(&identity.key, descriptor),
            identity,
        });
    }
    custom_shader_frame_requests_fit(&requests).then_some(requests)
}

impl GpuSurfaceRenderer {
    pub(super) fn preflight_custom_shader_upload_actions(
        &self,
        target: super::upload_plan::GpuSurfaceRenderCanvasUploadTarget,
        surface_index: usize,
        surface: &PaintGpuSurface,
        updates: CustomShaderDataUpdates<'_>,
        state: &mut CustomShaderUploadPreflightState,
        actions: &mut Vec<GpuSurfaceRenderCanvasUploadAction>,
    ) -> CustomShaderUploadPreflight {
        let presentation_updates = updates.presentation;
        let persistent_storage = updates.persistent;
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
        if matches!(
            select_persistent_storage(
                persistent_storage,
                surface,
                descriptor,
                PersistentStorageBindingCursor::default()
            ),
            PersistentStorageSelection::Mismatch
        ) {
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
        let device = target.device;
        let identity = CustomShaderPipelineIdentity {
            device,
            format: target.format,
            key: pipeline_key.clone(),
        };
        if state.defer_transition || !self.custom_shader_preparation_available(&identity) {
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
        let Some(pipeline_rebuild) = state.pipeline_decision(surface.key, &identity) else {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index,
                key: surface.key,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            });
            return CustomShaderUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete),
            };
        };
        actions.push(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index,
            key: surface.key,
            surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        });
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
            state.binding_decision(surface.key, &binding_cache_key);
        actions.push(GpuSurfaceRenderCanvasUploadAction::CustomBinding {
            surface_index,
            key: surface.key,
            cache_key: binding_cache_key,
            rebuild: binding_rebuild,
        });

        let mut persistent_cursor = if binding_rebuild {
            PersistentStorageBindingCursor::default()
        } else {
            state
                .persistent_cursors
                .get(&surface.key)
                .copied()
                .unwrap_or_else(|| {
                    self.resources
                        .custom_shader_binding(surface.key)
                        .map_or_else(PersistentStorageBindingCursor::default, |binding| {
                            binding.persistent_storage_cursor
                        })
                })
        };
        let persistent_selection =
            select_persistent_storage(persistent_storage, surface, descriptor, persistent_cursor);
        let persistent_plan = match persistent_selection {
            PersistentStorageSelection::Upload { plan, .. } => Some(plan),
            PersistentStorageSelection::Absent => None,
            PersistentStorageSelection::Mismatch => unreachable!("validated before preflight"),
        };
        let restore_bulk = persistent_plan.is_none() && persistent_cursor.effective().is_some();
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
        let static_write = write_state.static_payload_needs_write(static_payload) || restore_bulk;
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
            if !descriptor.storage_bytes.is_empty() && persistent_plan.is_none() {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                    byte_len: descriptor.storage_bytes.len(),
                });
            }
            write_state.cache_static_payload(static_payload);
            state.cache_static_payload(surface.key, static_payload);
        }

        actions.push(
            GpuSurfaceRenderCanvasUploadAction::CustomPersistentStorage {
                surface_index,
                key: surface.key,
                plan: persistent_plan.clone(),
            },
        );
        if let Some(plan) = &persistent_plan {
            for range in &plan.ranges {
                actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                    surface_index,
                    class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                    byte_len: range.byte_len,
                });
            }
            persistent_cursor.stage(plan.desired);
        } else {
            persistent_cursor.stage_bulk_reset();
        }
        state
            .persistent_cursors
            .insert(surface.key, persistent_cursor);

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
        if matches!(
            select_persistent_storage(
                request.persistent_storage,
                surface,
                descriptor,
                PersistentStorageBindingCursor::default()
            ),
            PersistentStorageSelection::Mismatch
        ) {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            record_failed_custom_shader_surface(stats);
            return false;
        }
        if !self.prepare_custom_shader_resources(target, surface, descriptor, stats) {
            return false;
        }
        let presentation_update =
            matching_presentation_update(surface, descriptor, presentation_updates);
        let presentation_staging_belt = (!descriptor.storage_bytes.is_empty()
            || descriptor
                .presentation_uniform_bytes
                .as_ref()
                .is_some_and(|bytes| !bytes.is_empty()))
        .then(|| {
            self.presentation_staging_belt.get_or_insert_with(|| {
                vello::wgpu::util::StagingBelt::new(
                    target.device.clone(),
                    PRESENTATION_STAGING_BELT_CHUNK_SIZE,
                )
            })
        });
        {
            let Some(binding) = self.resources.custom_shader_binding_mut(surface.key) else {
                record_failed_custom_shader_surface(stats);
                return false;
            };
            if !draw::upload_custom_shader_buffers(
                CustomShaderBufferUploadRequest {
                    target,
                    surface_index: 0,
                    surface,
                    descriptor,
                    binding,
                    presentation_update,
                    presentation_updates,
                    persistent_storage: request.persistent_storage,
                    presentation_staging_belt,
                },
                stats,
            ) {
                record_failed_custom_shader_surface(stats);
                return false;
            }
        }
        let Some(pipeline) = self.resources.custom_shader_pipeline(surface.key) else {
            record_failed_custom_shader_surface(stats);
            return false;
        };
        let Some(binding) = self.resources.custom_shader_binding(surface.key) else {
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
        if let GpuSurfaceContent::CustomShader { descriptor } = &request.surface.content
            && custom_shader_descriptor_is_supported(descriptor)
            && request.surface.content.is_renderable()
            && let Some(key) = custom_shader_pipeline_key(descriptor)
        {
            let identity = CustomShaderPipelineIdentity {
                device: super::wgpu_device_id(target.device),
                format: target.format,
                key,
            };
            if self.defer_custom_shader_transition
                || !self.custom_shader_preparation_available(&identity)
            {
                Self::consume_terminal_surface_decision(
                    upload_plan,
                    request.surface_index,
                    request.surface.key,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    stats,
                );
                match self.custom_shader_preparation_failure(&identity) {
                    Some(pipeline::CustomShaderPreparationFailure::ShaderModule) => {
                        stats.custom_shader.failures.shader_module_failures += 1
                    }
                    Some(pipeline::CustomShaderPreparationFailure::Pipeline) => {
                        stats.custom_shader.failures.pipeline_failures += 1
                    }
                    _ => {}
                }
                record_failed_custom_shader_surface(stats);
                return false;
            }
        }
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
        if !self.ensure_custom_shader_pipeline(pipeline_request, stats) {
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
        if !self.resources.has_custom_shader_binding(surface.key) {
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
        let presentation_staging_belt = (!descriptor.storage_bytes.is_empty()
            || descriptor
                .presentation_uniform_bytes
                .as_ref()
                .is_some_and(|bytes| !bytes.is_empty()))
        .then(|| {
            self.presentation_staging_belt.get_or_insert_with(|| {
                vello::wgpu::util::StagingBelt::new(
                    target.device.clone(),
                    PRESENTATION_STAGING_BELT_CHUNK_SIZE,
                )
            })
        });
        let upload_succeeded = {
            let Some(binding) = self.resources.custom_shader_binding_mut(surface.key) else {
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
                    persistent_storage: request.persistent_storage,
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
        let Some(binding) = self.resources.custom_shader_binding(surface.key) else {
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
        if !self.ensure_custom_shader_pipeline(
            CustomShaderPipelineRequest {
                surface_key: surface.key,
                device: target.device,
                target_format: target.format,
                key: pipeline_key,
            },
            stats,
        ) {
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
        if self.resources.has_custom_shader_binding(surface.key) {
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

pub(in crate::gui_runtime::native_vello::generic_runtime) fn custom_shader_descriptor_is_supported(
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
        runtime::{GpuSurfaceCapabilities, PaintPrimitive},
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

    #[test]
    fn custom_shader_frame_requests_share_identity_text_and_preserve_order() {
        let source = custom_shader_source(600 * 1024);
        let primitives = [
            custom_shader_primitive(41, "shared", source.clone()),
            custom_shader_primitive(7, "shared", source),
        ];

        let requests =
            custom_shader_frame_requests(&primitives, 3, vello::wgpu::TextureFormat::Rgba8Unorm)
                .expect("one distinct 600 KiB identity fits the frame budget");

        assert_eq!(
            requests
                .iter()
                .map(|request| request.surface_key)
                .collect::<Vec<_>>(),
            [41, 7]
        );
        assert_eq!(requests[0].identity, requests[1].identity);
        assert!(Arc::ptr_eq(
            &requests[0].identity.key.wgsl_source,
            &requests[1].identity.key.wgsl_source,
        ));
        assert!(Arc::ptr_eq(
            &requests[0].binding_key.pipeline_key.shader_key,
            &requests[1].binding_key.pipeline_key.shader_key,
        ));
    }

    #[test]
    fn custom_shader_frame_requests_reject_distinct_identities_over_text_budget() {
        let source = custom_shader_source(600 * 1024);
        let mut distinct_source = source.clone();
        distinct_source.push('\n');
        let primitives = [
            custom_shader_primitive(1, "first", source),
            custom_shader_primitive(2, "second", distinct_source),
        ];

        assert!(
            custom_shader_frame_requests(&primitives, 3, vello::wgpu::TextureFormat::Rgba8Unorm,)
                .is_none()
        );
    }

    fn custom_shader_primitive(key: u64, shader_key: &str, source: String) -> PaintPrimitive {
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 17,
            key,
            revision: 2,
            rect: test_rect(),
            content: GpuSurfaceContent::CustomShader {
                descriptor: Arc::new(
                    GpuShaderSurfaceDescriptor::new(shader_key)
                        .wgsl_source(source)
                        .entry_point("vertex_main")
                        .fragment_entry_point("fragment_main"),
                ),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })
    }

    fn custom_shader_source(bytes: usize) -> String {
        let mut source =
            String::from("@vertex fn vertex_main() {}\n@fragment fn fragment_main() {}\n//");
        let padding = bytes.saturating_sub(source.len());
        source.push_str(&"x".repeat(padding));
        source
    }

    fn test_rect() -> Rect {
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 24.0))
    }
}
