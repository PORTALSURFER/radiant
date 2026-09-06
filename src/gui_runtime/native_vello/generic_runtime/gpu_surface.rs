//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::device::{wgpu_device_id, wgpu_target_matches};
use super::runtime_helpers::{
    SurfaceOcclusionPlan, SurfaceOcclusionPolicy, SurfaceOcclusionQueryScratch,
    planned_surface_occlusion_regions_into,
};
use super::{
    GpuSurfaceAtlasResidencySnapshot, GpuSurfaceCustomShaderResidencySnapshot,
    GpuSurfaceSignalResidencySnapshot,
};
use crate::gui::types::{Rect as UiRect, Vector2};
use crate::runtime::{GpuShaderPresentationUniformUpdate, GpuSurfaceContent, PaintPrimitive};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use vello::wgpu;

mod active_keys;
mod atlas;
mod custom_shader;
mod encoding;
mod gpu_surface_types;
mod identity;
mod overlays;
mod passes;
mod pipeline;
mod resources;
mod signal;
mod signal_pipeline;
mod stats;
mod upload_plan;
mod upload_scratch;
mod visibility;
use active_keys::ActiveGpuSurfaceKeys;
use atlas::AtlasRenderRequest;
use custom_shader::CustomShaderRenderRequest;
use gpu_surface_types::{GpuSurfacePipeline, SignalPipeline};
use resources::GpuSurfaceResourceCache;
#[cfg(test)]
pub(super) use signal_pipeline::GPU_SIGNAL_SHADER;
pub(super) use stats::GpuSurfaceRenderCanvasUploadStats;
pub(super) use stats::GpuSurfaceRenderStats;
use upload_plan::GpuSurfaceRenderCanvasUploadAction;
pub(super) use upload_plan::GpuSurfaceRenderCanvasUploadPlanUnavailableReason;
pub(super) use upload_plan::{
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanContext,
    GpuSurfaceRenderCanvasUploadPlanObservation, GpuSurfaceRenderCanvasUploadTarget,
};
use upload_scratch::GpuSurfaceRenderCanvasUploadScratch;
pub(super) use visibility::gpu_surface_visible_suffix_regions_into_with_scratch;
pub use visibility::{
    GpuSurfaceOcclusionPlanningScratch, plan_gpu_surface_occlusion_for_diagnostics,
};
pub(in crate::gui_runtime::native_vello) use visibility::{
    SurfaceVisibleSuffixScratch, gpu_surface_requires_compositing_in_viewport,
    surface_rect_has_visible_region_in_viewport,
};

const PRESENTATION_STAGING_BELT_CHUNK_SIZE: wgpu::BufferAddress = 4096;

enum UploadPlanExecution<'a> {
    NoPlan,
    Executing(&'a mut GpuSurfaceRenderCanvasUploadPlan),
    Vetoed { mutated: bool },
}

impl<'a> UploadPlanExecution<'a> {
    fn into_plan(self) -> Option<&'a mut GpuSurfaceRenderCanvasUploadPlan> {
        match self {
            Self::Executing(plan) => Some(plan),
            Self::NoPlan | Self::Vetoed { .. } => None,
        }
    }
}

fn upload_plan_for_execution(
    plan_started: bool,
    upload_plan: &mut Option<GpuSurfaceRenderCanvasUploadPlan>,
) -> UploadPlanExecution<'_> {
    if !plan_started {
        return UploadPlanExecution::NoPlan;
    }
    let Some(plan) = upload_plan.as_mut() else {
        return UploadPlanExecution::NoPlan;
    };
    if plan.execution_is_available() {
        UploadPlanExecution::Executing(plan)
    } else {
        UploadPlanExecution::Vetoed {
            mutated: plan.execution_mutated(),
        }
    }
}

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    pipeline_generation: u64,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    resources: GpuSurfaceResourceCache,
    active_keys: ActiveGpuSurfaceKeys,
    occlusion_regions: Vec<UiRect>,
    occlusion_query_scratch: SurfaceOcclusionQueryScratch,
    upload_scratch: GpuSurfaceRenderCanvasUploadScratch,
    presentation_staging_belt: Option<wgpu::util::StagingBelt>,
}

pub(super) struct GpuSurfaceRenderTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) format: wgpu::TextureFormat,
    pub(super) size: Vector2,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) upload_plan_context: Option<GpuSurfaceRenderCanvasUploadPlanContext>,
    pub(super) upload_plan: Option<GpuSurfaceRenderCanvasUploadPlan>,
    pub(super) collect_upload_plan: bool,
}

impl GpuSurfaceRenderer {
    pub(super) fn preflight_render_canvas_upload_plan_with_dpi_scale(
        &mut self,
        context: GpuSurfaceRenderCanvasUploadPlanContext,
        primitives: &[PaintPrimitive],
        dpi_scale: crate::theme::DpiScale,
        presentation_updates: &[GpuShaderPresentationUniformUpdate],
    ) -> GpuSurfaceRenderCanvasUploadPlan {
        let state_fingerprint =
            self.upload_plan_state_fingerprint_with_presentation_updates(presentation_updates);
        let actions = self.upload_scratch.take_action_stream();
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::preflight_with_actions(
            context,
            primitives.as_ptr() as usize,
            primitives.len(),
            state_fingerprint,
            actions,
        );
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);

        let atlas_pipeline = self
            .pipeline
            .as_ref()
            .map(|pipeline| (pipeline.device, pipeline.format, self.pipeline_generation));
        let signal_pipeline = self.signal_pipeline.as_ref().map(|pipeline| {
            (
                pipeline.device,
                pipeline.format,
                self.signal_pipeline_generation,
            )
        });
        let mut atlas_preflight_state = std::mem::take(&mut self.upload_scratch.atlas);
        atlas_preflight_state.reset(atlas_pipeline);
        let mut signal_preflight_state = std::mem::take(&mut self.upload_scratch.signal);
        signal_preflight_state.reset(signal_pipeline);
        let mut custom_shader_preflight_state =
            std::mem::take(&mut self.upload_scratch.custom_shader);
        let custom_requests = custom_shader::custom_shader_frame_requests(
            primitives,
            context.target.device,
            context.target.format,
        );
        let custom_transition = custom_requests.as_ref().is_some_and(|requests| {
            self.resources
                .custom_shader_frame_requires_transition(requests)
        });
        custom_shader_preflight_state.reset(custom_requests.as_ref().map(|requests| {
            self.resources
                .custom_shader_frame_preflight(requests, custom_transition)
        }));
        if custom_transition && let Some(requests) = custom_requests.as_ref() {
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::CustomShaderTransition {
                requests: requests.clone(),
            });
        }
        let mut has_active_keys = false;

        for (surface_index, primitive) in primitives.iter().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            let is_signal = matches!(
                &surface.content,
                GpuSurfaceContent::SignalBands { .. }
                    | GpuSurfaceContent::SignalSummaryBands { .. }
            );
            let is_custom_shader =
                matches!(&surface.content, GpuSurfaceContent::CustomShader { .. });
            if !surface.rect.has_finite_positive_area() {
                plan.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid);
                plan.push_action(GpuSurfaceRenderCanvasUploadAction::Skip {
                    surface_index,
                    key: surface.key,
                    reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid,
                });
                continue;
            }
            if is_custom_shader {
                let preflight = self.preflight_custom_shader_upload_actions(
                    context.target,
                    surface_index,
                    surface,
                    presentation_updates,
                    &mut custom_shader_preflight_state,
                    &mut plan.actions,
                );
                if let Some(reason) = preflight.unavailable {
                    plan.mark_unavailable(reason);
                }
                if preflight.renderable {
                    has_active_keys = true;
                    plan.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                        surface_index,
                        key: surface.key,
                    });
                }
                continue;
            }
            if is_signal {
                let preflight = self.preflight_signal_upload_actions(
                    context.target,
                    dpi_scale,
                    surface_index,
                    surface,
                    signal::SignalUploadPreflightContext {
                        composite_state: &mut atlas_preflight_state,
                        signal_state: &mut signal_preflight_state,
                        actions: &mut plan.actions,
                    },
                );
                if let Some(reason) = preflight.unavailable {
                    plan.mark_unavailable(reason);
                }
                if preflight.renderable {
                    has_active_keys = true;
                    plan.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                        surface_index,
                        key: surface.key,
                    });
                }
                continue;
            }
            if matches!(&surface.content, GpuSurfaceContent::RgbaAtlas { .. })
                && !surface.content.is_renderable()
            {
                plan.mark_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
                );
                plan.push_action(GpuSurfaceRenderCanvasUploadAction::Skip {
                    surface_index,
                    key: surface.key,
                    reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
                });
                continue;
            }
            if matches!(&surface.content, GpuSurfaceContent::RgbaAtlas { .. }) {
                plan.enable_atlas_executor(true);
                match self.preflight_atlas_upload_actions(
                    context.target,
                    surface_index,
                    surface,
                    &mut atlas_preflight_state,
                    &mut plan.actions,
                ) {
                    Ok(()) => {}
                    Err(reason) => {
                        plan.mark_unavailable(reason);
                        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Skip {
                            surface_index,
                            key: surface.key,
                            reason,
                        });
                    }
                }
            }
            has_active_keys = true;
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index,
                key: surface.key,
            });
        }

        self.upload_scratch.atlas = atlas_preflight_state;
        self.upload_scratch.signal = signal_preflight_state;
        self.upload_scratch.custom_shader = custom_shader_preflight_state;
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Prune {
            clear: !has_active_keys,
        });
        plan
    }

    fn upload_plan_state_fingerprint(&mut self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.pipeline_generation.hash(&mut hasher);
        self.signal_pipeline_generation.hash(&mut hasher);
        self.upload_scratch.fingerprint.reset();
        self.resources
            .hash_atlas_state(&mut hasher, &mut self.upload_scratch.fingerprint);
        self.resources.textures.len().hash(&mut hasher);
        self.resources.composite_bindings.len().hash(&mut hasher);
        self.resources
            .custom_shader_pipeline_count()
            .hash(&mut hasher);
        self.resources
            .custom_shader_binding_count()
            .hash(&mut hasher);
        self.resources.signal_bodies.len().hash(&mut hasher);
        self.resources.signals.len().hash(&mut hasher);
        self.resources.signal_summaries.len().hash(&mut hasher);
        self.resources
            .signal_summary_validations
            .len()
            .hash(&mut hasher);
        hasher.finish()
    }

    fn upload_plan_state_fingerprint_with_presentation_updates(
        &mut self,
        presentation_updates: &[GpuShaderPresentationUniformUpdate],
    ) -> u64 {
        if presentation_updates.is_empty() {
            return self.upload_plan_state_fingerprint();
        }
        let mut hasher = DefaultHasher::new();
        self.upload_plan_state_fingerprint().hash(&mut hasher);
        for update in presentation_updates {
            update.widget_id.hash(&mut hasher);
            update.surface_key.get().hash(&mut hasher);
            update.storage_identity.hash(&mut hasher);
            update.storage_revision.hash(&mut hasher);
            update.presentation_revision.hash(&mut hasher);
            update.bytes().hash(&mut hasher);
        }
        hasher.finish()
    }

    fn commit_active_key(
        &mut self,
        plan_in_flight: bool,
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        activate: GpuSurfaceRenderCanvasUploadAction,
        key: u64,
        activation_already_consumed: bool,
    ) -> bool {
        let activation_succeeded = if !plan_in_flight {
            true
        } else if activation_already_consumed {
            upload_plan
                .as_ref()
                .is_some_and(|plan| plan.execution_is_available())
        } else {
            upload_plan.is_some_and(|plan| plan.consume_action(activate))
        };
        if activation_succeeded {
            self.active_keys.mark_active(key);
        }
        activation_succeeded
    }

    fn finish_resource_cleanup(
        &mut self,
        plan_in_flight: bool,
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
    ) -> bool {
        let clear = self.active_keys.is_empty();
        if plan_in_flight {
            let Some(plan) = upload_plan else {
                return false;
            };
            if !plan.consume_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear }) {
                return false;
            }
        }
        if !self.active_keys.is_empty() {
            self.prune_inactive_resources();
        } else {
            self.clear_resources();
        }
        true
    }

    fn consume_terminal_surface_decision(
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        surface_index: usize,
        key: u64,
        fallback_reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        let Some(upload_plan) = upload_plan else {
            stats.mark_candidate_unavailable(fallback_reason);
            return None;
        };
        match upload_plan.consume_surface_decision(surface_index, key) {
            Some(Err(reason)) => {
                stats.mark_candidate_unavailable(reason);
                Some(reason)
            }
            Some(Ok(_)) | None => {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                None
            }
        }
    }

    pub(super) fn render(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        primitives: &[PaintPrimitive],
        occlusion_plan: &SurfaceOcclusionPlan,
        presentation_updates: &[GpuShaderPresentationUniformUpdate],
    ) -> GpuSurfaceRenderStats {
        let mut upload_plan = target.upload_plan.take();
        let plan_started = upload_plan.as_mut().is_none_or(|plan| {
            let Some(context) = target.upload_plan_context else {
                plan.mark_unavailable(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid);
                return false;
            };
            plan.begin_execution(
                context,
                primitives.as_ptr() as usize,
                primitives.len(),
                self.upload_plan_state_fingerprint_with_presentation_updates(presentation_updates),
            )
        });
        let mut plan_in_flight = plan_started && upload_plan.is_some();
        if plan_in_flight
            && let Some(plan) = upload_plan.as_mut()
            && !plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame)
        {
            plan_in_flight = false;
        }
        let candidate_context = target
            .collect_upload_plan
            .then_some(target.upload_plan_context)
            .flatten();
        let mut stats = GpuSurfaceRenderStats::with_upload_plan(candidate_context);
        if !plan_started
            && let Some(plan) = upload_plan.as_ref()
            && let GpuSurfaceRenderCanvasUploadPlanObservation::Unavailable(reason) =
                plan.observation()
        {
            stats.mark_candidate_unavailable(reason);
        }
        let custom_requests = custom_shader::custom_shader_frame_requests(
            primitives,
            wgpu_device_id(target.device),
            target.format,
        );
        let custom_transition = custom_requests.as_ref().is_some_and(|requests| {
            self.resources
                .custom_shader_frame_requires_transition(requests)
        });
        let mut transition_authorized = true;
        if custom_transition && let Some(requests) = custom_requests.as_ref() {
            if let Some(plan) = upload_plan.as_mut() {
                transition_authorized = plan_in_flight
                    && plan.consume_action(
                        GpuSurfaceRenderCanvasUploadAction::CustomShaderTransition {
                            requests: requests.clone(),
                        },
                    );
            }
            if transition_authorized {
                transition_authorized = self.resources.begin_custom_shader_transition(requests);
                if transition_authorized && let Some(plan) = upload_plan.as_mut() {
                    plan.mark_execution_mutated();
                }
            }
            if !transition_authorized {
                if let Some(plan) = upload_plan.as_mut() {
                    plan.veto_execution(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                }
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            }
        }
        let mut occlusion_regions = std::mem::take(&mut self.occlusion_regions);
        self.active_keys.begin_frame();
        for (index, primitive) in primitives.iter().enumerate() {
            if !transition_authorized {
                break;
            }
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            let surface_upload_plan =
                match upload_plan_for_execution(plan_started, &mut upload_plan) {
                    UploadPlanExecution::Vetoed { mutated: true } => break,
                    execution @ UploadPlanExecution::Vetoed { mutated: false } => {
                        plan_in_flight = false;
                        execution.into_plan()
                    }
                    execution => execution.into_plan(),
                };
            if !surface.rect.has_finite_positive_area() {
                Self::consume_terminal_surface_decision(
                    surface_upload_plan,
                    index,
                    surface.key,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid,
                    &mut stats,
                );
                continue;
            }
            if custom_requests.is_none()
                && let GpuSurfaceContent::CustomShader { descriptor } = &surface.content
                && surface.content.is_renderable()
                && custom_shader::custom_shader_descriptor_is_supported(descriptor)
            {
                Self::consume_terminal_surface_decision(
                    surface_upload_plan,
                    index,
                    surface.key,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    &mut stats,
                );
                stats.custom_shader.failures.surfaces_failed += 1;
                continue;
            }
            let is_renderable = match &surface.content {
                GpuSurfaceContent::SignalBands { .. }
                | GpuSurfaceContent::SignalSummaryBands { .. } => true,
                GpuSurfaceContent::RgbaAtlas { .. } => {
                    if !surface.content.is_renderable() {
                        Self::consume_terminal_surface_decision(
                            surface_upload_plan,
                            index,
                            surface.key,
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
                            &mut stats,
                        );
                        continue;
                    }
                    true
                }
                GpuSurfaceContent::CustomShader { .. } => {
                    if !surface.content.is_renderable() {
                        Self::consume_terminal_surface_decision(
                            surface_upload_plan,
                            index,
                            surface.key,
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
                            &mut stats,
                        );
                        if let GpuSurfaceContent::CustomShader { descriptor } = &surface.content {
                            custom_shader::record_unsupported_custom_shader(descriptor, &mut stats);
                        }
                        continue;
                    }
                    true
                }
            };
            if !is_renderable {
                continue;
            }
            planned_surface_occlusion_regions_into(
                surface.rect,
                index,
                occlusion_plan,
                SurfaceOcclusionPolicy::GpuCompositor,
                &mut occlusion_regions,
                &mut self.occlusion_query_scratch,
            );
            let active = match &surface.content {
                GpuSurfaceContent::RgbaAtlas { source_rect, .. } => {
                    let atlas_upload_plan = surface_upload_plan
                        .and_then(|plan| plan.atlas_executor_enabled().then_some(plan));
                    self.render_atlas(
                        target,
                        AtlasRenderRequest {
                            surface_index: index,
                            surface,
                            source_rect: *source_rect,
                            occlusion_regions: &occlusion_regions,
                        },
                        atlas_upload_plan,
                        &mut stats,
                    )
                }
                GpuSurfaceContent::SignalBands { .. } => self.render_signal(
                    target,
                    index,
                    surface,
                    &occlusion_regions,
                    surface_upload_plan,
                    &mut stats,
                ),
                GpuSurfaceContent::SignalSummaryBands { .. } => self.render_signal(
                    target,
                    index,
                    surface,
                    &occlusion_regions,
                    surface_upload_plan,
                    &mut stats,
                ),
                GpuSurfaceContent::CustomShader { .. } => self.render_custom_shader(
                    target,
                    CustomShaderRenderRequest {
                        surface_index: index,
                        surface,
                        occlusion_regions: &occlusion_regions,
                        presentation_updates,
                    },
                    surface_upload_plan,
                    &mut stats,
                ),
            };
            if plan_in_flight
                && let Some(plan) = upload_plan.as_ref()
                && !plan.execution_is_available()
            {
                if plan.execution_mutated() {
                    break;
                }
                plan_in_flight = false;
            }
            if !active {
                continue;
            }
            let activate = GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: index,
                key: surface.key,
            };
            let activation_already_consumed =
                matches!(&surface.content, GpuSurfaceContent::CustomShader { .. });
            if !self.commit_active_key(
                plan_in_flight,
                upload_plan.as_mut(),
                activate,
                surface.key,
                activation_already_consumed,
            ) {
                continue;
            }
        }
        let cleanup_succeeded = transition_authorized
            && self.finish_resource_cleanup(plan_in_flight, upload_plan.as_mut());
        self.occlusion_regions = occlusion_regions;
        let mut transaction_complete = cleanup_succeeded;
        if let Some(mut plan) = upload_plan.take() {
            transaction_complete &= plan.finish_execution();
            self.upload_scratch.recycle_plan(plan);
        }
        self.resources
            .finish_custom_shader_transition(transaction_complete);
        stats
    }

    /// Close the mapped presentation staging chunks before the frame encoder
    /// is finished and submitted.
    pub(super) fn finish_presentation_staging_belt(&mut self) {
        if let Some(belt) = self.presentation_staging_belt.as_mut() {
            belt.finish();
        }
    }

    /// Return submitted presentation staging chunks to the reusable belt.
    pub(super) fn recall_presentation_staging_belt(&mut self) {
        if let Some(belt) = self.presentation_staging_belt.as_mut() {
            belt.recall();
        }
    }

    /// Drop presentation staging chunks when the frame cannot be submitted.
    pub(super) fn discard_presentation_staging_belt(&mut self) {
        self.presentation_staging_belt = None;
    }

    fn prune_inactive_resources(&mut self) {
        self.resources.prune_inactive(&self.active_keys);
    }

    fn clear_resources(&mut self) {
        self.resources.clear();
    }

    pub(super) fn atlas_residency_snapshot(&self) -> GpuSurfaceAtlasResidencySnapshot {
        self.resources.atlas_residency_snapshot()
    }

    pub(super) fn signal_residency_snapshot(&self) -> GpuSurfaceSignalResidencySnapshot {
        self.resources.signal_residency_snapshot()
    }

    pub(super) fn custom_shader_residency_snapshot(
        &self,
    ) -> GpuSurfaceCustomShaderResidencySnapshot {
        self.resources.custom_shader_residency_snapshot()
    }

    #[cfg(test)]
    fn collect_occlusion_regions_for_test(
        &mut self,
        surface_rect: UiRect,
        suffix: &[PaintPrimitive],
    ) -> &[UiRect] {
        let mut primitives = Vec::with_capacity(suffix.len() + 1);
        primitives.push(PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 0,
            rect: surface_rect,
            color: crate::gui::types::Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        }));
        primitives.extend_from_slice(suffix);
        let mut plan = SurfaceOcclusionPlan::default();
        plan.preprocess(&primitives);
        planned_surface_occlusion_regions_into(
            surface_rect,
            0,
            &plan,
            SurfaceOcclusionPolicy::GpuCompositor,
            &mut self.occlusion_regions,
            &mut self.occlusion_query_scratch,
        );
        &self.occlusion_regions
    }
}

#[cfg(test)]
#[path = "gpu_surface/custom_shader/native_tests.rs"]
mod native_tests;

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::identity::RenderCanvasContentIdentity;
    use super::resources::CachedSignalSummaryRequest;
    use super::upload_plan::{
        GpuSurfaceRenderCanvasUploadAtlasTextureOperation, GpuSurfaceRenderCanvasUploadClass,
        GpuSurfaceRenderCanvasUploadCompositeBindingOperation,
        GpuSurfaceRenderCanvasUploadPipeline, GpuSurfaceRenderCanvasUploadSignalBodyOperation,
        GpuSurfaceRenderCanvasUploadSignalBufferOperation,
        GpuSurfaceRenderCanvasUploadSignalSummaryOperation,
        GpuSurfaceRenderCanvasUploadSignalValidationOperation, GpuSurfaceRenderCanvasUploadSurface,
    };
    use super::*;
    use crate::gui::types::{Point, Rgba8};
    use crate::gui_runtime::native_vello::generic_runtime::FrameWork;
    use crate::gui_runtime::native_vello::generic_runtime::adapter::NativeAdapterGeneration;
    use crate::gui_runtime::native_vello::generic_runtime::closing::NativeLifecycle;
    use crate::gui_runtime::native_vello::generic_runtime::native_encode_present::{
        NativeEncodePresentPath, NativeEncodePresentPlanContext,
    };
    use crate::gui_runtime::native_vello::generic_runtime::native_visual_packet::{
        NativeVisualRequestAdapter, NativeVisualRequestBegin, NativeVisualRequestMailbox,
    };
    use crate::gui_runtime::native_vello::generic_runtime::runner_state::NativeTargetGeneration;
    use crate::runtime::{GpuSignalSummary, GpuSurfaceCapabilities, PaintGpuSurface};
    use std::sync::Arc;
    use winit::window::WindowId;

    fn upload_plan_context_for_test() -> GpuSurfaceRenderCanvasUploadPlanContext {
        upload_plan_context_for_test_with_generation(1)
    }

    fn upload_plan_context_for_test_with_generation(
        serial: u64,
    ) -> GpuSurfaceRenderCanvasUploadPlanContext {
        let mut mailbox = NativeVisualRequestMailbox::new();
        let window_id = WindowId::dummy();
        assert!(mailbox.bind_window(window_id));
        let _ = mailbox.enqueue_for_test(FrameWork::None);
        let packet = match NativeVisualRequestAdapter::begin(&mut mailbox, window_id, true) {
            NativeVisualRequestBegin::Requested(packet) => packet.identity(),
            other => panic!("unexpected packet begin: {other:?}"),
        };
        let generation = NativeAdapterGeneration::from_test_serial(serial);
        GpuSurfaceRenderCanvasUploadPlanContext::new(
            NativeEncodePresentPlanContext {
                packet,
                adapter_generation: generation,
                target_generation: NativeTargetGeneration::from_test_serial(1),
                lifecycle: NativeLifecycle::default(),
                path: NativeEncodePresentPath::Composited,
                snapshot_revision: NonZeroU64::MIN,
            },
            generation,
            GpuSurfaceRenderCanvasUploadTarget::new(1, wgpu::TextureFormat::Rgba8Unorm, 64, 32),
        )
        .expect("valid upload-plan context")
    }

    #[test]
    fn rejected_upload_plan_selects_legacy_family_path_without_consuming_actions() {
        let context = upload_plan_context_for_test();
        let stream = [0_u8; 1];
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            7,
        );
        plan.enable_atlas_executor(true);
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        let mut upload_plan = Some(plan);

        assert!(!upload_plan.as_mut().expect("upload plan").begin_execution(
            context,
            stream.as_ptr() as usize,
            stream.len() + 1,
            7,
        ));
        assert!(matches!(
            upload_plan_for_execution(false, &mut upload_plan),
            UploadPlanExecution::NoPlan
        ));
        assert!(
            upload_plan
                .as_ref()
                .is_some_and(|plan| !plan.execution_is_available())
        );
        assert!(matches!(
            upload_plan.as_ref().expect("upload plan").actions.first(),
            Some(GpuSurfaceRenderCanvasUploadAction::BeginFrame)
        ));
        assert!(
            !upload_plan
                .as_mut()
                .expect("upload plan")
                .consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame)
        );

        let mut no_plan = None;
        assert!(matches!(
            upload_plan_for_execution(true, &mut no_plan),
            UploadPlanExecution::NoPlan
        ));
    }

    #[test]
    fn post_write_veto_skips_later_mixed_surface_families_without_legacy_work() {
        let context = upload_plan_context_for_test();
        let stream = [0_u8; 1];
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            0,
        );
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index: 0,
            key: 41,
            surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        });
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: 4,
        });
        let mut upload_plan = Some(plan);

        assert!(upload_plan.as_mut().expect("upload plan").begin_execution(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            0,
        ));
        assert!(
            upload_plan
                .as_mut()
                .expect("upload plan")
                .consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame)
        );
        assert!(
            upload_plan
                .as_mut()
                .expect("upload plan")
                .consume_surface_decision(0, 41)
                .is_some()
        );
        assert_eq!(
            upload_plan
                .as_mut()
                .expect("upload plan")
                .consume_upload(0, GpuSurfaceRenderCanvasUploadClass::RendererParameter),
            Some(4)
        );
        upload_plan
            .as_mut()
            .expect("upload plan")
            .mark_execution_mutated();
        upload_plan
            .as_mut()
            .expect("upload plan")
            .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);

        let mut legacy_work = Vec::new();
        for surface in [
            GpuSurfaceRenderCanvasUploadSurface::Atlas,
            GpuSurfaceRenderCanvasUploadSurface::Signal,
            GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        ] {
            match upload_plan_for_execution(true, &mut upload_plan) {
                UploadPlanExecution::Vetoed { mutated: true } => break,
                UploadPlanExecution::NoPlan | UploadPlanExecution::Vetoed { mutated: false } => {
                    legacy_work.push(surface)
                }
                UploadPlanExecution::Executing(_) => {
                    panic!("a vetoed transaction must not resume execution")
                }
            }
        }
        assert!(legacy_work.is_empty());
    }

    #[test]
    fn render_canvas_preflight_walks_the_ordered_stream_without_live_mutation() {
        let mut renderer = GpuSurfaceRenderer::default();
        let before_fingerprint = renderer.upload_plan_state_fingerprint();
        let before_occlusion = renderer.occlusion_regions.clone();
        let before_active_capacity = renderer.active_keys.capacity();
        let empty_plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            upload_plan_context_for_test(),
            &[],
            crate::theme::DpiScale::ONE,
            &[],
        );
        assert_eq!(
            empty_plan.observation(),
            GpuSurfaceRenderCanvasUploadPlanObservation::NoWork
        );
        assert_eq!(
            empty_plan.actions,
            vec![
                GpuSurfaceRenderCanvasUploadAction::BeginFrame,
                GpuSurfaceRenderCanvasUploadAction::Prune { clear: true },
            ]
        );
        let atlas = Arc::new(
            crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                .expect("valid one-pixel image"),
        );
        let primitives = vec![
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 1,
                rect: crate::gui::types::Rect::from_min_size(
                    Point::new(0.0, 0.0),
                    Vector2::new(8.0, 8.0),
                ),
                color: Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 41,
                revision: 7,
                rect: crate::gui::types::Rect::from_min_size(
                    Point::new(2.0, 2.0),
                    Vector2::new(4.0, 4.0),
                ),
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: crate::gui::types::Rect::from_min_size(
                        Point::new(0.0, 0.0),
                        Vector2::new(1.0, 1.0),
                    ),
                    atlas,
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
        ];
        let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            upload_plan_context_for_test(),
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );

        assert!(plan.atlas_executor_enabled());
        assert_eq!(plan.actions.len(), 9);
        assert!(matches!(
            &plan.actions[0],
            GpuSurfaceRenderCanvasUploadAction::BeginFrame
        ));
        assert!(matches!(
            &plan.actions[1],
            GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: 1,
                key: 41,
                surface: GpuSurfaceRenderCanvasUploadSurface::Atlas,
            }
        ));
        assert!(matches!(
            &plan.actions[2],
            GpuSurfaceRenderCanvasUploadAction::AtlasTexture {
                surface_index: 1,
                key: 41,
                device: 1,
                revision: 7,
                width: 1,
                height: 1,
                byte_len: 4,
                extent_width: 1,
                extent_height: 1,
                bytes_per_row: 4,
                operation: GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
                    revision_mismatch: false,
                    content_mismatch: false,
                },
                ..
            }
        ));
        assert_eq!(
            &plan.actions[3],
            &GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 1,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len: 4,
            }
        );
        assert_eq!(
            &plan.actions[4],
            &GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
                pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
                device: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                generation: 1,
                rebuild: true,
            }
        );
        assert!(matches!(
            &plan.actions[5],
            GpuSurfaceRenderCanvasUploadAction::CompositeBinding {
                surface_index: 1,
                key: 41,
                uniform_byte_len: 240,
                operation: GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild {
                    revision_mismatch: false,
                    content_mismatch: false,
                },
                ..
            }
        ));
        assert_eq!(
            &plan.actions[6],
            &GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 1,
                class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
                byte_len: 240,
            }
        );
        assert_eq!(
            &plan.actions[7],
            &GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: 1,
                key: 41,
            }
        );
        assert!(matches!(
            &plan.actions[8],
            GpuSurfaceRenderCanvasUploadAction::Prune { clear: false }
        ));
        assert_eq!(renderer.upload_plan_state_fingerprint(), before_fingerprint);
        assert_eq!(renderer.occlusion_regions, before_occlusion);
        assert_eq!(renderer.active_keys.capacity(), before_active_capacity);
        assert!(renderer.active_keys.is_empty());
        assert!(renderer.resources.textures.is_empty());
        assert!(renderer.resources.composite_bindings.is_empty());
        assert!(renderer.resources.signal_summaries.is_empty());
    }

    #[test]
    fn signal_preflight_records_ordered_actions_without_live_mutation() {
        let mut renderer = GpuSurfaceRenderer::default();
        let context = upload_plan_context_for_test();
        let before_fingerprint = renderer.upload_plan_state_fingerprint();
        let samples: Arc<[f32]> = [0.0, 1.0, -0.5, 0.25].into();
        let primitives = [PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 2,
            key: 73,
            revision: 11,
            rect: UiRect::from_min_size(Point::new(2.0, 2.0), Vector2::new(4.0, 4.0)),
            content: GpuSurfaceContent::SignalBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                samples,
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })];

        let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );

        assert_eq!(plan.actions.len(), 14);
        assert!(matches!(
            &plan.actions[0],
            GpuSurfaceRenderCanvasUploadAction::BeginFrame
        ));
        assert!(matches!(
            &plan.actions[1],
            GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: 0,
                key: 73,
                surface: GpuSurfaceRenderCanvasUploadSurface::Signal,
            }
        ));
        assert!(matches!(
            &plan.actions[2],
            GpuSurfaceRenderCanvasUploadAction::SignalValidation {
                surface_index: 0,
                key: 73,
                valid: true,
                operation: GpuSurfaceRenderCanvasUploadSignalValidationOperation::Pure,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[3],
            GpuSurfaceRenderCanvasUploadAction::SignalSummary {
                surface_index: 0,
                key: 73,
                operation: GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Build,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[4],
            GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
                pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
                device: 1,
                generation: 1,
                rebuild: true,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[5],
            GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
                pipeline: GpuSurfaceRenderCanvasUploadPipeline::Signal,
                device: 1,
                generation: 1,
                rebuild: true,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[6],
            GpuSurfaceRenderCanvasUploadAction::SignalBuffer {
                surface_index: 0,
                key: 73,
                operation: GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[7],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[8],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[9],
            GpuSurfaceRenderCanvasUploadAction::SignalBody {
                surface_index: 0,
                key: 73,
                operation: GpuSurfaceRenderCanvasUploadSignalBodyOperation::Render,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[10],
            GpuSurfaceRenderCanvasUploadAction::CompositeBinding {
                surface_index: 0,
                key: 73,
                operation: GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild { .. },
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[11],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[12],
            GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: 0,
                key: 73,
            }
        ));
        assert!(matches!(
            &plan.actions[13],
            GpuSurfaceRenderCanvasUploadAction::Prune { clear: false }
        ));
        assert_eq!(renderer.upload_plan_state_fingerprint(), before_fingerprint);
        assert!(renderer.signal_pipeline.is_none());
        assert!(renderer.resources.signals.is_empty());
        assert!(renderer.resources.signal_bodies.is_empty());
        assert!(renderer.resources.signal_summaries.is_empty());
        assert!(renderer.resources.signal_summary_validations.is_empty());
        assert!(renderer.resources.composite_bindings.is_empty());
        let signal_residency = renderer.signal_residency_snapshot();
        assert_eq!(signal_residency.signal_buffer_resident_count, 0);
        assert_eq!(signal_residency.signal_buffer_logical_bytes, Some(0));
        assert_eq!(signal_residency.signal_body_texture_resident_count, 0);
        assert_eq!(
            signal_residency.signal_body_texture_logical_rgba_bytes,
            Some(0)
        );
    }

    #[test]
    fn custom_shader_preflight_records_ordered_actions_without_live_mutation() {
        let mut renderer = GpuSurfaceRenderer::default();
        let context = upload_plan_context_for_test();
        let before_fingerprint = renderer.upload_plan_state_fingerprint();
        let descriptor = Arc::new(
            crate::runtime::GpuShaderSurfaceDescriptor::new("test/custom-shader")
                .wgsl_source(
                    "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
                )
                .entry_point("vertex_main")
                .fragment_entry_point("fragment_main")
                .uniform_bytes([1, 2, 3, 4])
                .storage_bytes([5, 6, 7, 8])
                .presentation_uniform([9, 10, 11, 12], 2),
        );
        let primitives = [PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 5,
            key: 75,
            revision: 13,
            rect: UiRect::from_min_size(Point::new(2.0, 2.0), Vector2::new(4.0, 4.0)),
            content: GpuSurfaceContent::CustomShader { descriptor },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })];

        let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );

        assert!(matches!(
            &plan.actions[0],
            GpuSurfaceRenderCanvasUploadAction::BeginFrame
        ));
        assert!(matches!(
            &plan.actions[1],
            GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: 0,
                key: 75,
                surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
            }
        ));
        assert!(matches!(
            &plan.actions[2],
            GpuSurfaceRenderCanvasUploadAction::CustomPipeline {
                surface_index: 0,
                key: 75,
                device: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                rebuild: true,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[3],
            GpuSurfaceRenderCanvasUploadAction::CustomBinding {
                surface_index: 0,
                key: 75,
                rebuild: true,
                ..
            }
        ));
        assert_eq!(
            plan.actions[4],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
                byte_len: 240,
            }
        );
        assert!(matches!(
            &plan.actions[5],
            GpuSurfaceRenderCanvasUploadAction::CustomStaticState {
                surface_index: 0,
                key: 75,
                write: true,
                ..
            }
        ));
        assert_eq!(
            plan.actions[6],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len: 4,
            }
        );
        assert_eq!(
            plan.actions[7],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len: 4,
            }
        );
        assert!(matches!(
            &plan.actions[8],
            GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                source: super::upload_plan::GpuSurfaceRenderCanvasUploadCustomPresentationSource::Initial,
                revision: 2,
                byte_len: 4,
                write: true,
                ..
            }
        ));
        assert_eq!(
            plan.actions[9],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
                byte_len: 4,
            }
        );
        assert!(matches!(
            &plan.actions[10],
            GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                source:
                    super::upload_plan::GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update,
                revision: 0,
                byte_len: 0,
                write: false,
                ..
            }
        ));
        assert!(matches!(
            &plan.actions[11],
            GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: 0,
                key: 75,
            }
        ));
        assert!(matches!(
            &plan.actions[12],
            GpuSurfaceRenderCanvasUploadAction::Prune { clear: false }
        ));
        assert_eq!(renderer.upload_plan_state_fingerprint(), before_fingerprint);
        assert!(renderer.resources.custom_shader_pipelines_are_empty());
        assert!(renderer.resources.custom_shader_bindings_are_empty());
        assert!(renderer.active_keys.is_empty());
    }

    #[test]
    fn custom_shader_preflight_matches_presentation_updates_after_initial_state() {
        let mut renderer = GpuSurfaceRenderer::default();
        let context = upload_plan_context_for_test();
        let descriptor = Arc::new(
            crate::runtime::GpuShaderSurfaceDescriptor::new("test/custom-shader")
                .wgsl_source(
                    "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
                )
                .entry_point("vertex_main")
                .fragment_entry_point("fragment_main")
                .storage_identity(11)
                .storage_revision(13)
                .storage_bytes([5, 6, 7, 8])
                .presentation_uniform([9, 10, 11, 12], 2),
        );
        let primitives = [PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 5,
            key: 75,
            revision: 13,
            rect: UiRect::from_min_size(Point::new(2.0, 2.0), Vector2::new(4.0, 4.0)),
            content: GpuSurfaceContent::CustomShader { descriptor },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })];
        let update =
            GpuShaderPresentationUniformUpdate::try_new(5, 75, 11, 13, 7, [13, 14, 15, 16])
                .expect("valid presentation update");

        let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[update],
        );

        assert!(matches!(
            &plan.actions[9],
            GpuSurfaceRenderCanvasUploadAction::CustomPresentationState {
                source:
                    super::upload_plan::GpuSurfaceRenderCanvasUploadCustomPresentationSource::Update,
                revision: 7,
                byte_len: 4,
                write: true,
                ..
            }
        ));
        assert_eq!(
            plan.actions[10],
            GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index: 0,
                class: GpuSurfaceRenderCanvasUploadClass::VolatilePayload,
                byte_len: 4,
            }
        );
    }

    #[test]
    fn mixed_atlas_unsupported_custom_shader_and_signal_consumes_one_skip_and_prunes() {
        let mut renderer = GpuSurfaceRenderer::default();
        let context = upload_plan_context_for_test();
        let atlas = Arc::new(
            crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                .expect("valid one-pixel image"),
        );
        let unsupported_descriptor = Arc::new(
            crate::runtime::GpuShaderSurfaceDescriptor::new("test/unsupported-custom-shader")
                .uniform_bytes([1, 2, 3, 4]),
        );
        let samples: Arc<[f32]> = [0.0, 1.0, -0.5, 0.25].into();
        let primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 41,
                revision: 7,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 8.0)),
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: UiRect::from_min_size(
                        Point::new(0.0, 0.0),
                        Vector2::new(1.0, 1.0),
                    ),
                    atlas,
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 42,
                revision: 8,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 8.0)),
                content: GpuSurfaceContent::CustomShader {
                    descriptor: Arc::clone(&unsupported_descriptor),
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 3,
                key: 43,
                revision: 9,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 8.0)),
                content: GpuSurfaceContent::SignalBands {
                    frames: 4,
                    band_count: 1,
                    frame_range: [0.0, 4.0],
                    samples,
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
        ];
        let mut stale_stats = GpuSurfaceRenderStats::default();
        let stale_samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into();
        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 900,
            revision: 1,
            content_identity: RenderCanvasContentIdentity::default(),
            frames: 4,
            band_count: 1,
            samples: &stale_samples,
            stats: &mut stale_stats,
        });

        let stream_ptr = primitives.as_ptr() as usize;
        let stream_len = primitives.len();
        let mut plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        let custom_decision_count = plan
            .actions
            .iter()
            .filter(|action| {
                matches!(
                    action,
                    GpuSurfaceRenderCanvasUploadAction::Surface {
                        surface_index: 1,
                        ..
                    } | GpuSurfaceRenderCanvasUploadAction::Skip {
                        surface_index: 1,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(custom_decision_count, 1);
        assert!(plan.actions.iter().any(|action| matches!(
            action,
            GpuSurfaceRenderCanvasUploadAction::Skip {
                surface_index: 1,
                key: 42,
                reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Unsupported,
            }
        )));
        assert!(plan.actions.iter().any(|action| matches!(
            action,
            GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: 2,
                key: 43,
                surface: GpuSurfaceRenderCanvasUploadSurface::Signal,
            }
        )));
        assert!(matches!(
            plan.actions.last(),
            Some(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false })
        ));

        assert!(plan.begin_execution(
            context,
            stream_ptr,
            stream_len,
            renderer.upload_plan_state_fingerprint_with_presentation_updates(&[])
        ));
        assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        renderer.active_keys.begin_frame();
        let actions = plan.actions.iter().skip(1).cloned().collect::<Vec<_>>();
        let mut stats = GpuSurfaceRenderStats::default();
        let mut prior_mutation = false;
        let mut signal_decision_consumed = false;
        for action in actions {
            match action {
                GpuSurfaceRenderCanvasUploadAction::BeginFrame => {
                    panic!("begin-frame action should be consumed exactly once")
                }
                GpuSurfaceRenderCanvasUploadAction::Surface {
                    surface_index,
                    key,
                    surface,
                } => {
                    let decision = plan
                        .consume_surface_decision(surface_index, key)
                        .expect("surface decision");
                    assert_eq!(decision.expect("renderable surface").surface, surface);
                    if surface_index == 0 {
                        plan.mark_execution_mutated();
                        prior_mutation = true;
                    }
                    if surface_index == 2 {
                        signal_decision_consumed = true;
                    }
                }
                GpuSurfaceRenderCanvasUploadAction::Skip {
                    surface_index,
                    key,
                    reason,
                } => {
                    assert_eq!(surface_index, 1);
                    let consumed_reason = GpuSurfaceRenderer::consume_terminal_surface_decision(
                        Some(&mut plan),
                        surface_index,
                        key,
                        reason,
                        &mut stats,
                    );
                    assert_eq!(consumed_reason, Some(reason));
                    custom_shader::record_unsupported_custom_shader(
                        unsupported_descriptor.as_ref(),
                        &mut stats,
                    );
                    assert!(plan.execution_is_available());
                }
                GpuSurfaceRenderCanvasUploadAction::Activate { surface_index, key } => {
                    assert!(
                        plan.consume_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                            surface_index,
                            key,
                        })
                    );
                    renderer.active_keys.mark_active(key);
                }
                GpuSurfaceRenderCanvasUploadAction::Prune { clear } => {
                    assert!(!clear);
                    assert!(renderer.finish_resource_cleanup(true, Some(&mut plan)));
                }
                action => assert!(plan.consume_action(action)),
            }
        }
        assert!(prior_mutation);
        assert!(signal_decision_consumed);
        assert_eq!(stats.custom_shader.unsupported.surfaces, 1);
        assert!(!renderer.resources.signal_summaries.contains_key(&900));
        let custom_shader_residency = renderer.custom_shader_residency_snapshot();
        assert_eq!(custom_shader_residency.pipeline_resident_count, 0);
        assert_eq!(custom_shader_residency.binding_resident_count, 0);
        assert_eq!(
            custom_shader_residency.surface_uniform_logical_bytes,
            Some(0)
        );
        assert_eq!(custom_shader_residency.app_uniform_logical_bytes, Some(0));
        assert_eq!(custom_shader_residency.storage_logical_bytes, Some(0));
        assert_eq!(
            custom_shader_residency.presentation_uniform_logical_bytes,
            Some(0)
        );
        assert!(plan.finish_execution());
    }

    #[test]
    fn empty_gpu_surface_frame_consumes_begin_and_clears_prior_resources() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into();
        let mut stats = GpuSurfaceRenderStats::default();
        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            content_identity: RenderCanvasContentIdentity::default(),
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });
        renderer.active_keys.mark_active(7);

        let primitives: [PaintPrimitive; 0] = [];
        renderer.active_keys.begin_frame();
        let context = upload_plan_context_for_test();
        let mut plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        assert_eq!(
            plan.actions,
            vec![
                GpuSurfaceRenderCanvasUploadAction::BeginFrame,
                GpuSurfaceRenderCanvasUploadAction::Prune { clear: true },
            ]
        );
        assert!(plan.begin_execution(
            context,
            primitives.as_ptr() as usize,
            primitives.len(),
            renderer.upload_plan_state_fingerprint_with_presentation_updates(&[]),
        ));
        assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        assert!(renderer.finish_resource_cleanup(true, Some(&mut plan)));
        assert!(plan.finish_execution());
        assert!(renderer.resources.signal_summaries.is_empty());
        assert!(renderer.resources.signal_summary_validations.is_empty());
        assert!(renderer.active_keys.is_empty());
    }

    #[test]
    fn malformed_signal_preflight_vetoes_render_without_live_validation_mutation() {
        let mut renderer = GpuSurfaceRenderer::default();
        let context = upload_plan_context_for_test();
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            &[0.0, 1.0, -0.5, 0.25],
            2,
            2,
        ));
        let primitives = [PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 3,
            key: 74,
            revision: 12,
            rect: UiRect::from_min_size(Point::new(2.0, 2.0), Vector2::new(4.0, 4.0)),
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: 2,
                band_count: 1,
                frame_range: [0.0, 2.0],
                summary,
                gain_preview: None,
                sample_slide_frame_offset: 0,
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })];
        let stream_ptr = primitives.as_ptr() as usize;
        let stream_len = primitives.len();
        let state_fingerprint = renderer.upload_plan_state_fingerprint();
        let mut plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );

        assert_eq!(plan.actions.len(), 4);
        assert!(matches!(
            &plan.actions[2],
            GpuSurfaceRenderCanvasUploadAction::SignalValidation {
                valid: false,
                operation: GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheUpdate,
                ..
            }
        ));
        assert!(
            !plan.actions.iter().any(|action| matches!(
                action,
                GpuSurfaceRenderCanvasUploadAction::Activate { .. }
            ))
        );
        assert_eq!(renderer.resources.signal_summary_validations.len(), 0);

        assert!(plan.begin_execution(context, stream_ptr, stream_len, state_fingerprint));
        assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        assert!(plan.consume_surface_decision(0, 74).is_some());
        let validation = plan
            .consume_signal_validation(0, 74)
            .expect("signal validation action");
        assert!(!validation.valid);
        assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: true }));
        assert!(plan.finish_execution());
        assert!(plan.actions.is_empty());
        assert!(!plan.begin_execution(context, stream_ptr, stream_len, state_fingerprint));
        assert!(renderer.resources.signal_summary_validations.is_empty());
    }

    #[test]
    fn custom_shader_post_write_action_failure_aborts_without_legacy_replay() {
        let context = upload_plan_context_for_test();
        let stream = [0_u8; 1];
        let mut plan = GpuSurfaceRenderCanvasUploadPlan::preflight(
            context,
            stream.as_ptr() as usize,
            stream.len(),
            0,
        );
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index: 0,
            key: 41,
            surface: GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        });
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index: 0,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: 240,
        });
        plan.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
            surface_index: 0,
            key: 41,
        });

        assert!(plan.begin_execution(context, stream.as_ptr() as usize, stream.len(), 0));
        assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
        assert!(plan.consume_surface_decision(0, 41).is_some());
        assert_eq!(
            plan.consume_upload(0, GpuSurfaceRenderCanvasUploadClass::RendererParameter),
            Some(240)
        );
        plan.mark_execution_mutated();

        assert!(plan.consume_custom_static_state(0, 41).is_none());
        let outcome = super::custom_shader::custom_shader_plan_failure_result(&plan);
        assert_eq!(outcome, Some(false));

        let mut legacy_uploads = 0;
        let mut legacy_draws = 0;
        if outcome.is_none() {
            legacy_uploads += 1;
            legacy_draws += 1;
        }
        assert_eq!((legacy_uploads, legacy_draws), (0, 0));
    }

    #[test]
    fn mid_transaction_surface_failures_do_not_publish_active_keys_or_cleanup() {
        for surface in [
            GpuSurfaceRenderCanvasUploadSurface::Atlas,
            GpuSurfaceRenderCanvasUploadSurface::Signal,
            GpuSurfaceRenderCanvasUploadSurface::CustomShader,
        ] {
            let mut renderer = GpuSurfaceRenderer::default();
            let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
            let mut stats = GpuSurfaceRenderStats::default();
            renderer.cached_signal_summary(CachedSignalSummaryRequest {
                key: 99,
                revision: 1,
                content_identity: RenderCanvasContentIdentity::default(),
                frames: 4,
                band_count: 1,
                samples: &samples,
                stats: &mut stats,
            });

            let context = upload_plan_context_for_test();
            let stream = [0_u8; 1];
            let mut plan = GpuSurfaceRenderCanvasUploadPlan::preflight(
                context,
                stream.as_ptr() as usize,
                stream.len(),
                0,
            );
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame);
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::Surface {
                surface_index: 0,
                key: 41,
                surface,
            });
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::Activate {
                surface_index: 0,
                key: 41,
            });
            plan.push_action(GpuSurfaceRenderCanvasUploadAction::Prune { clear: false });

            assert!(plan.begin_execution(context, stream.as_ptr() as usize, stream.len(), 0));
            assert!(plan.consume_action(GpuSurfaceRenderCanvasUploadAction::BeginFrame));
            assert!(plan.consume_surface_decision(0, 41).is_some());
            plan.veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);

            assert!(!renderer.commit_active_key(
                true,
                Some(&mut plan),
                GpuSurfaceRenderCanvasUploadAction::Activate {
                    surface_index: 0,
                    key: 41,
                },
                41,
                false,
            ));
            assert!(!renderer.active_keys.contains(&41));
            assert!(!renderer.finish_resource_cleanup(true, Some(&mut plan)));
            assert!(renderer.resources.signal_summaries.contains_key(&99));
        }
    }

    #[test]
    fn gpu_surface_renderer_prunes_inactive_signal_summaries() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            content_identity: RenderCanvasContentIdentity::default(),
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });
        renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 8,
            revision: 1,
            content_identity: RenderCanvasContentIdentity::default(),
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });

        renderer.active_keys.mark_active(8);
        renderer.prune_inactive_resources();

        assert!(!renderer.resources.signal_summaries.contains_key(&7));
        assert!(renderer.resources.signal_summaries.contains_key(&8));
    }

    #[test]
    fn gpu_surface_renderer_prunes_every_resource_map_to_active_keys() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        let summary = renderer.cached_signal_summary(CachedSignalSummaryRequest {
            key: 7,
            revision: 1,
            content_identity: RenderCanvasContentIdentity::default(),
            frames: 4,
            band_count: 1,
            samples: &samples,
            stats: &mut stats,
        });
        let surface = crate::runtime::PaintGpuSurface {
            widget_id: 7,
            key: 7,
            revision: 1,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                summary,
                gain_preview: None,
                sample_slide_frame_offset: 0,
            },
            capabilities: Default::default(),
            overlays: Vec::new(),
        };
        assert!(
            renderer
                .validated_signal_render_shape(&surface, &mut stats)
                .is_some()
        );

        renderer.prune_inactive_resources();

        assert!(renderer.resources.textures.is_empty());
        assert!(renderer.resources.composite_bindings.is_empty());
        assert!(renderer.resources.signal_bodies.is_empty());
        assert!(renderer.resources.signals.is_empty());
        assert!(renderer.resources.signal_summaries.is_empty());
        assert!(renderer.resources.signal_summary_validations.is_empty());
    }

    #[test]
    fn mixed_upload_scratch_reuses_capacity_and_decisions_for_100_cycles() {
        let mut renderer = GpuSurfaceRenderer::default();
        let atlas = Arc::new(
            crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                .expect("valid one-pixel image"),
        );
        let samples: Arc<[f32]> = [0.0, 1.0, -0.5, 0.25].into();
        let descriptor = Arc::new(
            crate::runtime::GpuShaderSurfaceDescriptor::new("test/scratch-custom-shader")
                .wgsl_source(
                    "@vertex fn vertex_main() -> @builtin(position) vec4<f32> { return vec4<f32>(); }\n@fragment fn fragment_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
                )
                .entry_point("vertex_main")
                .fragment_entry_point("fragment_main")
                .uniform_bytes([1, 2, 3, 4])
                .storage_bytes([5, 6, 7, 8]),
        );
        let primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 41,
                revision: 7,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(4.0, 4.0)),
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: UiRect::from_min_size(
                        Point::new(0.0, 0.0),
                        Vector2::new(1.0, 1.0),
                    ),
                    atlas,
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 43,
                revision: 11,
                rect: UiRect::from_min_size(Point::new(4.0, 0.0), Vector2::new(4.0, 4.0)),
                content: GpuSurfaceContent::SignalBands {
                    frames: 4,
                    band_count: 1,
                    frame_range: [0.0, 4.0],
                    samples,
                },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 3,
                key: 45,
                revision: 13,
                rect: UiRect::from_min_size(Point::new(8.0, 0.0), Vector2::new(4.0, 4.0)),
                content: GpuSurfaceContent::CustomShader { descriptor },
                capabilities: GpuSurfaceCapabilities::default(),
                overlays: Vec::new(),
            }),
        ];
        let context = upload_plan_context_for_test();
        let action_evidence = |actions: &[GpuSurfaceRenderCanvasUploadAction]| {
            actions
                .iter()
                .fold([(0_usize, 0_u64); 3], |mut evidence, action| {
                    let GpuSurfaceRenderCanvasUploadAction::Upload {
                        class, byte_len, ..
                    } = action
                    else {
                        return evidence;
                    };
                    let index = match class {
                        GpuSurfaceRenderCanvasUploadClass::ImmutablePayload => 0,
                        GpuSurfaceRenderCanvasUploadClass::VolatilePayload => 1,
                        GpuSurfaceRenderCanvasUploadClass::RendererParameter => 2,
                    };
                    evidence[index].0 += 1;
                    evidence[index].1 += *byte_len as u64;
                    evidence
                })
        };

        let first = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        let expected_actions = first.actions.clone();
        let expected_evidence = action_evidence(&expected_actions);
        renderer.upload_scratch.recycle_plan(first);

        let warm = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        assert_eq!(warm.actions, expected_actions);
        assert_eq!(action_evidence(&warm.actions), expected_evidence);
        renderer.upload_scratch.recycle_plan(warm);
        let warm_capacities = renderer.upload_scratch.capacities();

        for _ in 0..100 {
            let plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
                context,
                &primitives,
                crate::theme::DpiScale::ONE,
                &[],
            );
            assert_eq!(plan.actions, expected_actions);
            assert_eq!(action_evidence(&plan.actions), expected_evidence);
            renderer.upload_scratch.recycle_plan(plan);
            assert_eq!(renderer.upload_scratch.capacities(), warm_capacities);
        }

        let mut stale_plan = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            context,
            &primitives,
            crate::theme::DpiScale::ONE,
            &[],
        );
        let next_context = upload_plan_context_for_test_with_generation(2);
        assert!(!stale_plan.begin_execution(
            next_context,
            primitives.as_ptr() as usize,
            primitives.len(),
            renderer.upload_plan_state_fingerprint(),
        ));
        renderer.upload_scratch.recycle_plan(stale_plan);

        let empty = renderer.preflight_render_canvas_upload_plan_with_dpi_scale(
            next_context,
            &[],
            crate::theme::DpiScale::ONE,
            &[],
        );
        assert_eq!(
            empty.actions,
            vec![
                GpuSurfaceRenderCanvasUploadAction::BeginFrame,
                GpuSurfaceRenderCanvasUploadAction::Prune { clear: true },
            ]
        );
        renderer.upload_scratch.recycle_plan(empty);
    }

    #[test]
    fn gpu_surface_renderer_reuses_occlusion_scratch_storage() {
        let mut renderer = GpuSurfaceRenderer {
            occlusion_regions: Vec::with_capacity(8),
            ..GpuSurfaceRenderer::default()
        };
        let capacity = renderer.occlusion_regions.capacity();
        let surface_rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 7,
            rect: UiRect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0)),
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        })];

        assert_eq!(
            renderer
                .collect_occlusion_regions_for_test(surface_rect, &suffix)
                .len(),
            1
        );
        let query_capacity = renderer.occlusion_query_scratch.capacity();
        assert!(
            renderer
                .collect_occlusion_regions_for_test(surface_rect, &[])
                .is_empty()
        );

        assert_eq!(renderer.occlusion_regions.capacity(), capacity);
        assert_eq!(renderer.occlusion_query_scratch.capacity(), query_capacity);
    }
}
