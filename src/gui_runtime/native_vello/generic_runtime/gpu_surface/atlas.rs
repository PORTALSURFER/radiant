use super::encoding::uniforms_as_bytes;
use super::gpu_surface_types::{
    GpuSurfaceCompositeBinding, GpuSurfaceCompositeBindingKey, GpuSurfaceTextureIdentity,
    GpuSurfaceUniforms,
};
use super::overlays::vertical_overlays;
use super::passes::{gpu_surface_render_pass, set_surface_scissor, surface_dest};
use super::stats::GpuSurfaceRenderStats;
use super::upload_plan::{
    GpuSurfaceRenderCanvasUploadAction, GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
    GpuSurfaceRenderCanvasUploadAtlasTextureOperation, GpuSurfaceRenderCanvasUploadClass,
    GpuSurfaceRenderCanvasUploadCompositeBindingOperation, GpuSurfaceRenderCanvasUploadPipeline,
    GpuSurfaceRenderCanvasUploadPlan, GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    GpuSurfaceRenderCanvasUploadSurface, GpuSurfaceRenderCanvasUploadTarget,
};
use super::visibility::visible_surface_regions;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
use crate::gui::types::Rect as UiRect;
use crate::runtime::PaintGpuSurface;
use std::collections::HashMap;
use std::time::Instant;
use vello::wgpu;

pub(super) struct TextureViewRenderRequest<'a> {
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) texture_identity: GpuSurfaceTextureIdentity,
    pub(super) texture_view: &'a wgpu::TextureView,
    pub(super) source: [f32; 4],
    pub(super) occlusion_regions: &'a [UiRect],
}

pub(super) struct AtlasRenderRequest<'a> {
    pub(super) surface_index: usize,
    pub(super) surface: &'a PaintGpuSurface,
    pub(super) source_rect: UiRect,
    pub(super) occlusion_regions: &'a [UiRect],
}

#[derive(Clone, Copy)]
struct AtlasTexturePreflightIdentity {
    device: usize,
    revision: u64,
    content_identity: super::identity::RenderCanvasContentIdentity,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy)]
struct AtlasPipelinePreflightIdentity {
    device: usize,
    format: wgpu::TextureFormat,
    generation: u64,
}

#[derive(Default)]
pub(super) struct AtlasUploadPreflightState {
    pipeline: Option<AtlasPipelinePreflightIdentity>,
    textures: HashMap<u64, Option<AtlasTexturePreflightIdentity>>,
    composite_bindings: HashMap<u64, Option<GpuSurfaceCompositeBindingKey>>,
}

impl AtlasUploadPreflightState {
    pub(super) fn reset(&mut self, pipeline: Option<(usize, wgpu::TextureFormat, u64)>) {
        self.pipeline =
            pipeline.map(
                |(device, format, generation)| AtlasPipelinePreflightIdentity {
                    device,
                    format,
                    generation,
                },
            );
        self.textures.clear();
        self.composite_bindings.clear();
    }

    #[cfg(test)]
    pub(super) fn textures_capacity(&self) -> usize {
        self.textures.capacity()
    }

    #[cfg(test)]
    pub(super) fn composite_bindings_capacity(&self) -> usize {
        self.composite_bindings.capacity()
    }

    fn atlas_texture_operation(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        surface: &PaintGpuSurface,
        texture: &GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
    ) -> GpuSurfaceRenderCanvasUploadAtlasTextureOperation {
        let cached = *self.textures.entry(surface.key).or_insert_with(|| {
            renderer
                .resources
                .textures
                .get(&surface.key)
                .map(|texture| AtlasTexturePreflightIdentity {
                    device: texture.device,
                    revision: texture.revision,
                    content_identity: texture.content_identity,
                    width: texture.width,
                    height: texture.height,
                })
        });
        match cached {
            Some(cached)
                if cached.device == texture.device
                    && cached.revision == texture.revision
                    && cached.content_identity == texture.content_identity
                    && cached.width == texture.width
                    && cached.height == texture.height =>
            {
                GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Reuse
            }
            Some(cached) => GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
                revision_mismatch: cached.revision != texture.revision,
                content_mismatch: cached.revision == texture.revision,
            },
            None => GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
                revision_mismatch: false,
                content_mismatch: false,
            },
        }
    }

    pub(super) fn ensure_pipeline(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        device: usize,
        format: wgpu::TextureFormat,
    ) -> (u64, bool) {
        if let Some(pipeline) = self.pipeline
            && pipeline.device == device
            && pipeline.format == format
        {
            return (pipeline.generation, false);
        }
        let generation = renderer.pipeline_generation.wrapping_add(1);
        self.pipeline = Some(AtlasPipelinePreflightIdentity {
            device,
            format,
            generation,
        });
        (generation, true)
    }

    pub(super) fn composite_binding_operation(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        surface_key: u64,
        cache_key: GpuSurfaceCompositeBindingKey,
    ) -> GpuSurfaceRenderCanvasUploadCompositeBindingOperation {
        let cached = *self
            .composite_bindings
            .entry(surface_key)
            .or_insert_with(|| {
                renderer
                    .resources
                    .composite_bindings
                    .get(&surface_key)
                    .map(|binding| binding.cache_key)
            });
        match cached {
            Some(cached) if cached == cache_key => {
                GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Reuse
            }
            Some(cached) => GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild {
                revision_mismatch: cached.pipeline_generation == cache_key.pipeline_generation
                    && cached.revision() != cache_key.revision(),
                content_mismatch: cached.pipeline_generation == cache_key.pipeline_generation
                    && cached.revision() == cache_key.revision()
                    && cached.texture != cache_key.texture,
            },
            None => GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild {
                revision_mismatch: false,
                content_mismatch: false,
            },
        }
    }

    pub(super) fn record_composite_binding(
        &mut self,
        surface_key: u64,
        cache_key: GpuSurfaceCompositeBindingKey,
        _operation: GpuSurfaceRenderCanvasUploadCompositeBindingOperation,
    ) {
        self.composite_bindings.insert(surface_key, Some(cache_key));
    }

    fn record_texture(
        &mut self,
        surface_key: u64,
        texture: &GpuSurfaceRenderCanvasUploadAtlasTextureExecution,
    ) {
        self.textures.insert(
            surface_key,
            Some(AtlasTexturePreflightIdentity {
                device: texture.device,
                revision: texture.revision,
                content_identity: texture.content_identity,
                width: texture.width,
                height: texture.height,
            }),
        );
    }
}

impl GpuSurfaceRenderer {
    pub(super) fn preflight_atlas_upload_actions(
        &self,
        target: GpuSurfaceRenderCanvasUploadTarget,
        surface_index: usize,
        surface: &PaintGpuSurface,
        state: &mut AtlasUploadPreflightState,
        actions: &mut Vec<GpuSurfaceRenderCanvasUploadAction>,
    ) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        let mut texture = self.preflight_atlas_texture(target.device, surface_index, surface)?;
        texture.operation = state.atlas_texture_operation(self, surface, &texture);
        let (pipeline_generation, pipeline_rebuild) =
            state.ensure_pipeline(self, target.device, target.format);
        let texture_identity = GpuSurfaceTextureIdentity::RgbaAtlas {
            revision: texture.revision,
            content_identity: texture.content_identity,
            width: texture.width,
            height: texture.height,
        };
        let cache_key = GpuSurfaceCompositeBindingKey {
            pipeline_generation,
            texture: texture_identity,
        };
        let binding_operation = state.composite_binding_operation(self, surface.key, cache_key);
        state.record_composite_binding(surface.key, cache_key, binding_operation);

        actions.push(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index,
            key: surface.key,
            surface: GpuSurfaceRenderCanvasUploadSurface::Atlas,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::AtlasTexture {
            surface_index,
            key: surface.key,
            device: texture.device,
            revision: texture.revision,
            content_identity: texture.content_identity,
            width: texture.width,
            height: texture.height,
            byte_len: texture.byte_len,
            extent_width: texture.extent_width,
            extent_height: texture.extent_height,
            bytes_per_row: texture.bytes_per_row,
            operation: texture.operation,
        });
        if let GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload { .. } = texture.operation
        {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len: texture.byte_len,
            });
        }
        actions.push(GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
            pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
            device: target.device,
            format: target.format,
            generation: pipeline_generation,
            rebuild: pipeline_rebuild,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::CompositeBinding {
            surface_index,
            key: surface.key,
            cache_key,
            uniform_byte_len: std::mem::size_of::<GpuSurfaceUniforms>(),
            operation: binding_operation,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: std::mem::size_of::<GpuSurfaceUniforms>(),
        });
        state.record_texture(surface.key, &texture);
        Ok(())
    }

    pub(super) fn render_atlas(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: AtlasRenderRequest<'_>,
        mut upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        if let Some(upload_plan) = upload_plan.as_deref_mut()
            && upload_plan.execution_is_available()
            && let Some(rendered) =
                self.render_atlas_with_plan(target, &request, upload_plan, stats)
        {
            return rendered;
        }
        if upload_plan
            .as_deref()
            .is_some_and(|plan| plan.execution_mutated())
        {
            return false;
        }
        self.render_atlas_legacy(target, &request, stats)
    }

    fn render_atlas_with_plan(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: &AtlasRenderRequest<'_>,
        upload_plan: &mut GpuSurfaceRenderCanvasUploadPlan,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<bool> {
        let surface_index = request.surface_index;
        let surface = request.surface;
        let source_rect = request.source_rect;
        let occlusion_regions = request.occlusion_regions;
        match upload_plan.consume_surface_decision(surface_index, surface.key) {
            Some(Ok(decision))
                if decision.surface == GpuSurfaceRenderCanvasUploadSurface::Atlas => {}
            Some(Err(reason)) => {
                upload_plan.veto_execution(reason);
                stats.mark_candidate_unavailable(reason);
                return None;
            }
            Some(Ok(_)) => {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return None;
            }
            None => {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return None;
            }
        }
        let Some(texture_execution) = upload_plan.consume_atlas_texture(surface_index, surface.key)
        else {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return None;
        };
        let texture_was_uploaded = matches!(
            texture_execution.operation,
            super::upload_plan::GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload { .. }
        );
        let upload_byte_len = match texture_execution.operation {
            super::upload_plan::GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Reuse => None,
            super::upload_plan::GpuSurfaceRenderCanvasUploadAtlasTextureOperation::Upload {
                ..
            } => {
                let Some(byte_len) = upload_plan.consume_upload(
                    surface_index,
                    GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                ) else {
                    upload_plan.veto_execution(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    stats.mark_candidate_unavailable(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    return None;
                };
                Some(byte_len)
            }
        };
        if texture_was_uploaded {
            upload_plan.mark_execution_mutated();
        }
        if let Err(reason) = self.execute_atlas_texture(
            target.device,
            target.queue,
            surface,
            texture_execution,
            upload_byte_len,
            stats,
        ) {
            upload_plan.veto_execution(reason);
            stats.mark_candidate_unavailable(reason);
            return None;
        }
        let Some(pipeline_execution) =
            upload_plan.consume_pipeline(GpuSurfaceRenderCanvasUploadPipeline::Composite)
        else {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return texture_was_uploaded.then_some(true);
        };
        if pipeline_execution.rebuild {
            upload_plan.mark_execution_mutated();
        }
        if !self.atlas_pipeline_execution_matches(target, pipeline_execution) {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return texture_was_uploaded.then_some(true);
        };
        if let Err(reason) = self.execute_atlas_pipeline(target, pipeline_execution) {
            upload_plan.veto_execution(reason);
            stats.mark_candidate_unavailable(reason);
            return Some(true);
        }
        let Some(texture) = self.resources.textures.get(&surface.key) else {
            upload_plan
                .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return Some(true);
        };
        let texture_identity = GpuSurfaceTextureIdentity::RgbaAtlas {
            revision: texture.revision,
            content_identity: texture.content_identity,
            width: texture.width,
            height: texture.height,
        };
        let texture_view = texture.view.clone();
        self.render_atlas_texture_view(
            target,
            surface_index,
            TextureViewRenderRequest {
                surface,
                texture_identity,
                texture_view: &texture_view,
                source: [
                    source_rect.min.x,
                    source_rect.min.y,
                    source_rect.width(),
                    source_rect.height(),
                ],
                occlusion_regions,
            },
            upload_plan,
            stats,
        );
        Some(true)
    }

    fn render_atlas_legacy(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: &AtlasRenderRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        let surface = request.surface;
        let source_rect = request.source_rect;
        let occlusion_regions = request.occlusion_regions;
        self.ensure_texture_legacy(target.device, target.queue, surface, stats);
        self.ensure_pipeline(target.device, target.format);
        let Some(texture) = self.resources.textures.get(&surface.key) else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return true;
        };
        let texture_identity = GpuSurfaceTextureIdentity::RgbaAtlas {
            revision: texture.revision,
            content_identity: texture.content_identity,
            width: texture.width,
            height: texture.height,
        };
        let texture_view = texture.view.clone();
        self.render_texture_view(
            target,
            TextureViewRenderRequest {
                surface,
                texture_identity,
                texture_view: &texture_view,
                source: [
                    source_rect.min.x,
                    source_rect.min.y,
                    source_rect.width(),
                    source_rect.height(),
                ],
                occlusion_regions,
            },
            stats,
        );
        true
    }

    fn execute_atlas_pipeline(
        &mut self,
        target: &GpuSurfaceRenderTarget<'_>,
        execution: super::upload_plan::GpuSurfaceRenderCanvasUploadPipelineExecution,
    ) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        if !self.atlas_pipeline_execution_matches(target, execution) {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        self.ensure_pipeline(target.device, target.format);
        if self.pipeline_generation != execution.generation
            || self
                .pipeline
                .as_ref()
                .is_none_or(|pipeline| !pipeline.matches_target(target.device, target.format))
        {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        Ok(())
    }

    fn atlas_pipeline_execution_matches(
        &self,
        target: &GpuSurfaceRenderTarget<'_>,
        execution: super::upload_plan::GpuSurfaceRenderCanvasUploadPipelineExecution,
    ) -> bool {
        let device = super::wgpu_device_id(target.device);
        let rebuild = self
            .pipeline
            .as_ref()
            .is_none_or(|pipeline| !pipeline.matches_target(target.device, target.format));
        let generation = if rebuild {
            self.pipeline_generation.wrapping_add(1)
        } else {
            self.pipeline_generation
        };
        execution.device == device
            && execution.format == target.format
            && execution.generation == generation
            && execution.rebuild == rebuild
    }

    fn render_atlas_texture_view(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface_index: usize,
        request: TextureViewRenderRequest<'_>,
        upload_plan: &mut GpuSurfaceRenderCanvasUploadPlan,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.render_texture_view_with_plan(
            target,
            request,
            Some((surface_index, upload_plan)),
            stats,
        );
    }

    pub(super) fn render_texture_view(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: TextureViewRenderRequest<'_>,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        self.render_texture_view_with_plan(target, request, None, stats);
    }

    pub(super) fn render_texture_view_with_plan(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        request: TextureViewRenderRequest<'_>,
        mut atlas_execution: Option<(usize, &mut GpuSurfaceRenderCanvasUploadPlan)>,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(pipeline) = self.pipeline.as_ref() else {
            if let Some((_, upload_plan)) = atlas_execution {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
            }
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return;
        };
        let surface = request.surface;
        let overlay_uniforms = vertical_overlays(&surface.overlays);
        let uniforms = GpuSurfaceUniforms {
            dest: surface_dest(surface, target.dpi_scale),
            source: request.source,
            target_size: [target.size.x.max(1.0), target.size.y.max(1.0)],
            _padding: [0.0; 2],
            overlay_ratios: overlay_uniforms.ratios,
            overlay_widths: overlay_uniforms.widths,
            overlay_colors: overlay_uniforms.colors,
        };
        let cache_key = GpuSurfaceCompositeBindingKey {
            pipeline_generation: self.pipeline_generation,
            texture: request.texture_identity,
        };
        let expected_operation = self.atlas_composite_binding_operation(surface.key, cache_key);
        let rebuild_binding = if let Some((surface_index, upload_plan)) = atlas_execution.as_mut() {
            let Some(execution) =
                upload_plan.consume_composite_binding(*surface_index, surface.key)
            else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return;
            };
            if execution.cache_key != cache_key
                || execution.uniform_byte_len != std::mem::size_of::<GpuSurfaceUniforms>()
                || execution.operation != expected_operation
            {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return;
            }
            matches!(
                execution.operation,
                GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild { .. }
            )
        } else {
            matches!(
                expected_operation,
                GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild { .. }
            )
        };
        if rebuild_binding {
            if let Some((_, upload_plan)) = atlas_execution.as_mut() {
                upload_plan.mark_execution_mutated();
            }
            if let Some(binding) = self.resources.composite_bindings.get(&surface.key) {
                if binding.cache_key.pipeline_generation == cache_key.pipeline_generation
                    && binding.cache_key.revision() != cache_key.revision()
                {
                    stats.composite.binding_revision_mismatches += 1;
                } else if binding.cache_key.pipeline_generation == cache_key.pipeline_generation
                    && binding.cache_key.texture != cache_key.texture
                {
                    stats.composite.binding_content_mismatches += 1;
                }
            }
            // A composite bind group retains the body texture view independently
            // of the body cache; share that reservation until both owners drop.
            let signal_body = match request.texture_identity {
                GpuSurfaceTextureIdentity::SignalBody(key) => self
                    .resources
                    .signal_bodies
                    .get(&surface.key)
                    .filter(|body| body.cache_key == key),
                GpuSurfaceTextureIdentity::RgbaAtlas { .. } => None,
            };
            let signal_owner = signal_body.map(|body| body._content_owner.clone());
            let signal_body_lease = signal_body.and_then(|body| body._gpu_lease.clone());
            let signal_uniform_lease =
                if let Some(super::identity::RenderCanvasContentOwner::PreparedSignal(owner)) =
                    &signal_owner
                {
                    let Some(lease) = owner
                        .gpu_budget()
                        .reserve(std::mem::size_of::<GpuSurfaceUniforms>())
                    else {
                        if let Some((_, plan)) = atlas_execution.as_mut() {
                            plan.veto_execution(
                                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                            );
                        }
                        stats.mark_candidate_unavailable(
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        );
                        return;
                    };
                    Some(lease)
                } else {
                    None
                };
            stats.composite.binding_rebuilds += 1;
            let uniform_buffer = target.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("radiant_gpu_surface_uniforms"),
                size: std::mem::size_of::<GpuSurfaceUniforms>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = target.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("radiant_gpu_surface_bind_group"),
                layout: &pipeline.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(request.texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                    },
                ],
            });
            self.resources.composite_bindings.insert(
                surface.key,
                GpuSurfaceCompositeBinding {
                    cache_key,
                    uniform_buffer,
                    bind_group,
                    _signal_owner: signal_owner,
                    _signal_body_lease: signal_body_lease,
                    _signal_uniform_lease: signal_uniform_lease,
                },
            );
        } else {
            stats.composite.binding_cache_hits += 1;
        }
        let Some(binding) = self.resources.composite_bindings.get(&surface.key) else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return;
        };
        let uniform_bytes = uniforms_as_bytes(&uniforms);
        if let Some((surface_index, upload_plan)) = atlas_execution.as_mut() {
            let Some(byte_len) = upload_plan.consume_upload(
                *surface_index,
                GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            ) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return;
            };
            if byte_len != uniform_bytes.len() {
                upload_plan
                    .veto_execution(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return;
            }
        }
        stats.record_candidate_renderer_parameter(uniform_bytes.len());
        if let Some((_, upload_plan)) = atlas_execution.as_mut() {
            upload_plan.mark_execution_mutated();
        }
        target
            .queue
            .write_buffer(&binding.uniform_buffer, 0, uniform_bytes);
        stats
            .render_canvas_uploads
            .record_renderer_parameter(uniform_bytes.len());
        let started = Instant::now();
        let mut pass = gpu_surface_render_pass(target.encoder, target.target_view);
        pass.set_pipeline(&pipeline.pipeline);
        pass.set_bind_group(0, &binding.bind_group, &[]);
        for region in visible_surface_regions(surface.rect, request.occlusion_regions) {
            if set_surface_scissor(&mut pass, region, target.dpi_scale) {
                pass.draw(0..6, 0..1);
            }
        }
        stats.composite.encode_elapsed += started.elapsed();
    }

    pub(super) fn atlas_composite_binding_operation(
        &self,
        surface_key: u64,
        cache_key: GpuSurfaceCompositeBindingKey,
    ) -> GpuSurfaceRenderCanvasUploadCompositeBindingOperation {
        match self.resources.composite_bindings.get(&surface_key) {
            Some(binding) if binding.cache_key == cache_key => {
                GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Reuse
            }
            Some(binding) => GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild {
                revision_mismatch: binding.cache_key.pipeline_generation
                    == cache_key.pipeline_generation
                    && binding.cache_key.revision() != cache_key.revision(),
                content_mismatch: binding.cache_key.pipeline_generation
                    == cache_key.pipeline_generation
                    && binding.cache_key.revision() == cache_key.revision()
                    && binding.cache_key.texture != cache_key.texture,
            },
            None => GpuSurfaceRenderCanvasUploadCompositeBindingOperation::Rebuild {
                revision_mismatch: false,
                content_mismatch: false,
            },
        }
    }
}
