use super::atlas::AtlasUploadPreflightState;
use super::atlas::TextureViewRenderRequest;
use super::gpu_surface_types::{
    CachedSignalSummaryValidation, GpuSurfaceCompositeBindingKey, GpuSurfaceTextureIdentity,
    SignalBodyCacheKey, SignalBodyCacheKeyParts, SignalBufferCacheKey, SignalUniforms,
};
use super::identity::RenderCanvasContentOwner;
use super::identity::{RenderCanvasContentIdentity, SignalSourceIdentity};
use super::passes::surface_pixel_extent;
use super::stats::GpuSurfaceRenderStats;
use super::upload_plan::{
    GpuSurfaceRenderCanvasUploadAction, GpuSurfaceRenderCanvasUploadClass,
    GpuSurfaceRenderCanvasUploadPipeline, GpuSurfaceRenderCanvasUploadPlan,
    GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
    GpuSurfaceRenderCanvasUploadSignalBodyOperation,
    GpuSurfaceRenderCanvasUploadSignalBufferOperation,
    GpuSurfaceRenderCanvasUploadSignalSummaryOperation,
    GpuSurfaceRenderCanvasUploadSignalValidationOperation, GpuSurfaceRenderCanvasUploadTarget,
};
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
#[path = "signal/uniforms.rs"]
mod uniforms;
#[path = "signal/window.rs"]
mod window;
use super::encoding::{signal_uniforms_as_bytes, summary_bucket_bytes, summary_bucket_value_count};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket,
    GpuSignalSummaryLevel, GpuSurfaceContent, PaintGpuSurface,
};
use crate::theme::DpiScale;
use std::collections::HashMap;
use std::sync::Arc;
use uniforms::{signal_gain_preview, signal_sample_slide_frame_offset, signal_uniforms};
use vello::wgpu;
use window::{SignalBucketWindow, signal_bucket_window};

struct SignalRenderSource {
    shape: GpuSignalRenderShape,
    summary: Arc<GpuSignalSummary>,
    gain_preview: Option<GpuSignalGainPreview>,
    sample_slide_frame_offset: i64,
}

struct SignalBodyRequest<'a> {
    body_key: SignalBodyCacheKey,
    level_index: usize,
    bucket_start: usize,
    bucket_count: usize,
    buckets: &'a [GpuSignalSummaryBucket],
    uniforms: SignalUniforms,
}

struct SelectedSignalLevel<'a> {
    index: usize,
    level: &'a GpuSignalSummaryLevel,
    bucket_window: SignalBucketWindow,
}

struct SignalBodyKeyRequest<'a> {
    surface: &'a PaintGpuSurface,
    source: &'a SignalRenderSource,
    selected: &'a SelectedSignalLevel<'a>,
    dpi_scale: DpiScale,
}

#[derive(Clone)]
struct SignalSummaryPreflightIdentity {
    revision: u64,
    source_identity: SignalSourceIdentity,
    frames: usize,
    band_count: usize,
    sample_count: usize,
    summary: Arc<GpuSignalSummary>,
}

#[derive(Clone, Copy)]
struct SignalValidationPreflightIdentity {
    frames: usize,
    band_count: usize,
    summary: usize,
}

#[derive(Clone, Copy)]
struct SignalBufferPreflightIdentity {
    cache_key: SignalBufferCacheKey,
    sample_count: usize,
    pipeline_generation: u64,
}

#[derive(Clone, Copy)]
struct SignalBodyPreflightIdentity {
    device: usize,
    cache_key: SignalBodyCacheKey,
}

#[derive(Clone, Copy)]
struct SignalPipelinePreflightIdentity {
    device: usize,
    format: wgpu::TextureFormat,
    generation: u64,
}

#[derive(Default)]
pub(super) struct SignalUploadPreflightState {
    pipeline: Option<SignalPipelinePreflightIdentity>,
    validations: HashMap<u64, SignalValidationPreflightIdentity>,
    summaries: HashMap<u64, SignalSummaryPreflightIdentity>,
    buffers: HashMap<u64, SignalBufferPreflightIdentity>,
    bodies: HashMap<u64, SignalBodyPreflightIdentity>,
}

pub(super) struct SignalUploadPreflight {
    pub(super) renderable: bool,
    pub(super) unavailable: Option<GpuSurfaceRenderCanvasUploadPlanUnavailableReason>,
}

pub(super) struct SignalUploadPreflightContext<'a> {
    pub(super) composite_state: &'a mut AtlasUploadPreflightState,
    pub(super) signal_state: &'a mut SignalUploadPreflightState,
    pub(super) actions: &'a mut Vec<GpuSurfaceRenderCanvasUploadAction>,
}

impl SignalUploadPreflightState {
    pub(super) fn reset(&mut self, pipeline: Option<(usize, wgpu::TextureFormat, u64)>) {
        self.pipeline =
            pipeline.map(
                |(device, format, generation)| SignalPipelinePreflightIdentity {
                    device,
                    format,
                    generation,
                },
            );
        self.validations.clear();
        self.summaries.clear();
        self.buffers.clear();
        self.bodies.clear();
    }

    #[cfg(test)]
    pub(super) fn validations_capacity(&self) -> usize {
        self.validations.capacity()
    }

    #[cfg(test)]
    pub(super) fn summaries_capacity(&self) -> usize {
        self.summaries.capacity()
    }

    #[cfg(test)]
    pub(super) fn buffers_capacity(&self) -> usize {
        self.buffers.capacity()
    }

    #[cfg(test)]
    pub(super) fn bodies_capacity(&self) -> usize {
        self.bodies.capacity()
    }

    fn ensure_pipeline(
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
        let generation = renderer.signal_pipeline_generation.wrapping_add(1);
        self.pipeline = Some(SignalPipelinePreflightIdentity {
            device,
            format,
            generation,
        });
        (generation, true)
    }

    fn signal_summary(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        surface: &PaintGpuSurface,
        shape: GpuSignalRenderShape,
    ) -> (
        Arc<GpuSignalSummary>,
        GpuSurfaceRenderCanvasUploadSignalSummaryOperation,
    ) {
        let samples = match &surface.content {
            GpuSurfaceContent::SignalBands { samples, .. } => samples,
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                return (
                    Arc::clone(summary),
                    GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Reuse,
                );
            }
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => {
                return (
                    Arc::new(GpuSignalSummary {
                        frames: 0,
                        band_count: 0,
                        levels: Vec::new(),
                    }),
                    GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Build,
                );
            }
        };
        let source_identity = SignalSourceIdentity::from_content(&surface.content)
            .expect("signal summary preflight only receives signal content");
        let sample_count = samples.len();
        let cached = self.summaries.get(&surface.key).cloned().or_else(|| {
            renderer
                .resources
                .signal_summaries
                .get(&surface.key)
                .map(|cached| SignalSummaryPreflightIdentity {
                    revision: cached.revision,
                    source_identity: cached.source_identity,
                    frames: cached.frames,
                    band_count: cached.band_count,
                    sample_count: cached.sample_count,
                    summary: Arc::clone(&cached.summary),
                })
        });
        if let Some(cached) = cached
            && cached.revision == surface.revision
            && cached.source_identity == source_identity
            && cached.frames == shape.frames
            && cached.band_count == shape.band_count
            && cached.sample_count == sample_count
        {
            return (
                Arc::clone(&cached.summary),
                GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Reuse,
            );
        }
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            samples,
            shape.frames,
            shape.band_count,
        ));
        self.summaries.insert(
            surface.key,
            SignalSummaryPreflightIdentity {
                revision: surface.revision,
                source_identity,
                frames: shape.frames,
                band_count: shape.band_count,
                sample_count,
                summary: Arc::clone(&summary),
            },
        );
        (
            summary,
            GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Build,
        )
    }

    fn signal_buffer_operation(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        key: u64,
        cache_key: SignalBufferCacheKey,
        sample_count: usize,
        pipeline_generation: u64,
    ) -> GpuSurfaceRenderCanvasUploadSignalBufferOperation {
        let had_cached = self.buffers.contains_key(&key);
        let cached = self.buffers.entry(key).or_insert_with(|| {
            renderer
                .resources
                .signals
                .get(&key)
                .map(|buffer| SignalBufferPreflightIdentity {
                    cache_key: buffer.cache_key,
                    sample_count: buffer.sample_count,
                    pipeline_generation: buffer.pipeline_generation,
                })
                .unwrap_or(SignalBufferPreflightIdentity {
                    cache_key,
                    sample_count: 0,
                    pipeline_generation: u64::MAX,
                })
        });
        let operation = had_cached
            .then_some(())
            .filter(|_| {
                cached.cache_key == cache_key
                    && cached.sample_count == sample_count
                    && cached.pipeline_generation == pipeline_generation
            })
            .map(|_| GpuSurfaceRenderCanvasUploadSignalBufferOperation::Reuse)
            .unwrap_or(GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload);
        *cached = SignalBufferPreflightIdentity {
            cache_key,
            sample_count,
            pipeline_generation,
        };
        operation
    }

    fn signal_body_operation(
        &mut self,
        renderer: &GpuSurfaceRenderer,
        key: u64,
        device: usize,
        cache_key: SignalBodyCacheKey,
    ) -> GpuSurfaceRenderCanvasUploadSignalBodyOperation {
        let had_cached = self.bodies.contains_key(&key);
        let cached = self.bodies.entry(key).or_insert_with(|| {
            renderer
                .resources
                .signal_bodies
                .get(&key)
                .map(|body| SignalBodyPreflightIdentity {
                    device: body.device,
                    cache_key: body.cache_key,
                })
                .unwrap_or(SignalBodyPreflightIdentity { device, cache_key })
        });
        let operation = if had_cached && cached.device == device && cached.cache_key == cache_key {
            GpuSurfaceRenderCanvasUploadSignalBodyOperation::Reuse
        } else {
            GpuSurfaceRenderCanvasUploadSignalBodyOperation::Render
        };
        *cached = SignalBodyPreflightIdentity { device, cache_key };
        operation
    }
}

impl GpuSurfaceRenderer {
    pub(super) fn preflight_signal_upload_actions(
        &self,
        target: GpuSurfaceRenderCanvasUploadTarget,
        dpi_scale: DpiScale,
        surface_index: usize,
        surface: &PaintGpuSurface,
        context: SignalUploadPreflightContext<'_>,
    ) -> SignalUploadPreflight {
        let SignalUploadPreflightContext {
            composite_state,
            signal_state,
            actions,
        } = context;
        actions.push(GpuSurfaceRenderCanvasUploadAction::Surface {
            surface_index,
            key: surface.key,
            surface: super::upload_plan::GpuSurfaceRenderCanvasUploadSurface::Signal,
        });
        let (shape, validation_operation, summary_identity, validation_valid) = match &surface
            .content
        {
            GpuSurfaceContent::SignalBands { .. } => (
                surface.content.signal_render_shape(),
                GpuSurfaceRenderCanvasUploadSignalValidationOperation::Pure,
                0,
                surface.content.signal_render_shape().is_some(),
            ),
            GpuSurfaceContent::SignalSummaryBands {
                frames,
                band_count,
                summary,
                ..
            } => {
                let summary_identity = Arc::as_ptr(summary) as *const () as usize;
                let validation_operation = if signal_state
                    .validations
                    .get(&surface.key)
                    .is_some_and(|cached| {
                        cached.frames == *frames
                            && cached.band_count == *band_count
                            && cached.summary == summary_identity
                    }) {
                    GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheHit
                } else {
                    GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheUpdate
                };
                signal_state.validations.insert(
                    surface.key,
                    SignalValidationPreflightIdentity {
                        frames: *frames,
                        band_count: *band_count,
                        summary: summary_identity,
                    },
                );
                (
                    surface.content.signal_render_shape(),
                    validation_operation,
                    summary_identity,
                    surface.content.signal_render_shape().is_some(),
                )
            }
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => {
                return SignalUploadPreflight {
                    renderable: false,
                    unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
                };
            }
        };
        actions.push(GpuSurfaceRenderCanvasUploadAction::SignalValidation {
            surface_index,
            key: surface.key,
            frames: shape.map_or_else(
                || match &surface.content {
                    GpuSurfaceContent::SignalBands { frames, .. }
                    | GpuSurfaceContent::SignalSummaryBands { frames, .. } => *frames,
                    GpuSurfaceContent::RgbaAtlas { .. }
                    | GpuSurfaceContent::CustomShader { .. } => 0,
                },
                |shape| shape.frames,
            ),
            band_count: match &surface.content {
                GpuSurfaceContent::SignalBands { band_count, .. }
                | GpuSurfaceContent::SignalSummaryBands { band_count, .. } => *band_count,
                GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => 0,
            },
            summary: summary_identity,
            valid: validation_valid,
            operation: validation_operation,
        });
        let Some(shape) = shape else {
            return SignalUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
            };
        };

        let (summary, summary_operation) = match &surface.content {
            GpuSurfaceContent::SignalBands { .. } => {
                signal_state.signal_summary(self, surface, shape)
            }
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => (
                Arc::clone(summary),
                GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Reuse,
            ),
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => {
                return SignalUploadPreflight {
                    renderable: false,
                    unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
                };
            }
        };
        if matches!(&surface.content, GpuSurfaceContent::SignalBands { .. }) {
            actions.push(GpuSurfaceRenderCanvasUploadAction::SignalSummary {
                surface_index,
                key: surface.key,
                revision: surface.revision,
                source_identity: SignalSourceIdentity::from_content(&surface.content)
                    .expect("signal summary action only receives signal content"),
                frames: shape.frames,
                band_count: shape.band_count,
                sample_count: match &surface.content {
                    GpuSurfaceContent::SignalBands { samples, .. } => samples.len(),
                    _ => 0,
                },
                operation: summary_operation,
            });
        }
        let source = SignalRenderSource {
            shape,
            summary,
            gain_preview: signal_gain_preview(&surface.content),
            sample_slide_frame_offset: signal_sample_slide_frame_offset(&surface.content),
        };
        let Some(body) = signal_body_request_at_dpi(surface, &source, dpi_scale) else {
            return SignalUploadPreflight {
                renderable: false,
                unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete),
            };
        };
        let (pipeline_generation, pipeline_rebuild) =
            composite_state.ensure_pipeline(self, target.device, target.format);
        let (signal_pipeline_generation, signal_pipeline_rebuild) =
            signal_state.ensure_pipeline(self, target.device, wgpu::TextureFormat::Rgba8Unorm);
        let source_identity = SignalSourceIdentity::from_content(&surface.content)
            .expect("signal buffer preflight only receives signal content");
        let buffer_cache_key = SignalBufferCacheKey::new(
            surface.revision,
            source_identity,
            body.level_index,
            body.bucket_start,
            body.bucket_count,
        );
        let sample_count = summary_bucket_value_count(body.buckets);
        let buffer_operation = signal_state.signal_buffer_operation(
            self,
            surface.key,
            buffer_cache_key,
            sample_count,
            signal_pipeline_generation,
        );
        let body_operation =
            signal_state.signal_body_operation(self, surface.key, target.device, body.body_key);
        let texture_identity = GpuSurfaceTextureIdentity::SignalBody(body.body_key);
        let binding_cache_key = GpuSurfaceCompositeBindingKey {
            pipeline_generation,
            texture: texture_identity,
        };
        let binding_operation =
            composite_state.composite_binding_operation(self, surface.key, binding_cache_key);
        composite_state.record_composite_binding(surface.key, binding_cache_key, binding_operation);
        let immutable_byte_len = summary_bucket_bytes(body.buckets).len();
        let renderer_parameter_byte_len = signal_uniforms_as_bytes(&body.uniforms).len();
        actions.push(GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
            pipeline: GpuSurfaceRenderCanvasUploadPipeline::Composite,
            device: target.device,
            format: target.format,
            generation: pipeline_generation,
            rebuild: pipeline_rebuild,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::EnsurePipeline {
            pipeline: GpuSurfaceRenderCanvasUploadPipeline::Signal,
            device: target.device,
            format: wgpu::TextureFormat::Rgba8Unorm,
            generation: signal_pipeline_generation,
            rebuild: signal_pipeline_rebuild,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::SignalBuffer {
            surface_index,
            key: surface.key,
            cache_key: buffer_cache_key,
            sample_count,
            immutable_byte_len,
            renderer_parameter_byte_len,
            operation: buffer_operation,
        });
        if matches!(
            buffer_operation,
            GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload
        ) {
            actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
                surface_index,
                class: GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                byte_len: immutable_byte_len,
            });
        }
        actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: renderer_parameter_byte_len,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::SignalBody {
            surface_index,
            key: surface.key,
            device: target.device,
            cache_key: body.body_key,
            operation: body_operation,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::CompositeBinding {
            surface_index,
            key: surface.key,
            cache_key: binding_cache_key,
            uniform_byte_len: std::mem::size_of::<super::gpu_surface_types::GpuSurfaceUniforms>(),
            operation: binding_operation,
        });
        actions.push(GpuSurfaceRenderCanvasUploadAction::Upload {
            surface_index,
            class: GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            byte_len: std::mem::size_of::<super::gpu_surface_types::GpuSurfaceUniforms>(),
        });
        SignalUploadPreflight {
            renderable: true,
            unavailable: None,
        }
    }

    pub(super) fn render_signal(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface_index: usize,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> bool {
        let mut upload_plan = upload_plan;
        if upload_plan
            .as_deref()
            .is_some_and(|plan| !plan.execution_is_available())
        {
            if upload_plan
                .as_deref()
                .is_some_and(|plan| plan.execution_mutated())
            {
                return false;
            }
            upload_plan = None;
        }
        if let Some(plan) = upload_plan.as_deref_mut() {
            match plan.consume_surface_decision(surface_index, surface.key) {
                Some(Ok(decision))
                    if decision.surface
                        == super::upload_plan::GpuSurfaceRenderCanvasUploadSurface::Signal => {}
                Some(Err(reason)) => {
                    stats.mark_candidate_unavailable(reason);
                    return false;
                }
                Some(Ok(_)) => {
                    signal_plan_failure(
                        plan,
                        stats,
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    return false;
                }
                None => {
                    stats.mark_candidate_unavailable(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    return false;
                }
            }
        }
        let Some(shape) = self.validated_signal_render_shape_for_plan(
            surface,
            surface_index,
            upload_plan.as_deref_mut(),
            stats,
        ) else {
            return false;
        };
        let Some(source) = self.signal_render_source(
            surface,
            surface_index,
            shape,
            upload_plan.as_deref_mut(),
            stats,
        ) else {
            if let Some(plan) = upload_plan.as_deref_mut() {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            } else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            }
            return true;
        };
        let Some(body) = signal_body_request(target, surface, &source) else {
            if let Some(plan) = upload_plan.as_deref_mut() {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            } else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            }
            return true;
        };
        let source_identity = SignalSourceIdentity::from_content(&surface.content)
            .expect("signal buffer execution only receives signal content");
        let buffer_cache_key = SignalBufferCacheKey::new(
            surface.revision,
            source_identity,
            body.level_index,
            body.bucket_start,
            body.bucket_count,
        );
        if let Some(plan) = upload_plan {
            let Some(composite_pipeline) =
                plan.consume_pipeline(GpuSurfaceRenderCanvasUploadPipeline::Composite)
            else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            if composite_pipeline.rebuild {
                plan.mark_execution_mutated();
            }
            if let Err(reason) =
                self.execute_composite_pipeline_for_signal(target, composite_pipeline)
            {
                signal_plan_failure(plan, stats, reason);
                return true;
            }
            let Some(signal_pipeline) =
                plan.consume_pipeline(GpuSurfaceRenderCanvasUploadPipeline::Signal)
            else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            if signal_pipeline.rebuild {
                plan.mark_execution_mutated();
            }
            if let Err(reason) = self.execute_signal_pipeline_for_signal(target, signal_pipeline) {
                signal_plan_failure(plan, stats, reason);
                return true;
            }
            let sample_count = summary_bucket_value_count(body.buckets);
            let immutable_byte_len = summary_bucket_bytes(body.buckets).len();
            let renderer_parameter_byte_len = signal_uniforms_as_bytes(&body.uniforms).len();
            let Some(buffer_execution) = plan.consume_signal_buffer(surface_index, surface.key)
            else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            let expected_buffer_operation =
                self.signal_buffer_operation(surface.key, buffer_cache_key, sample_count);
            if buffer_execution.cache_key != buffer_cache_key
                || buffer_execution.sample_count != sample_count
                || buffer_execution.immutable_byte_len != immutable_byte_len
                || buffer_execution.renderer_parameter_byte_len != renderer_parameter_byte_len
                || buffer_execution.operation != expected_buffer_operation
            {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            }
            if matches!(
                buffer_execution.operation,
                GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload
            ) {
                let Some(byte_len) = plan.consume_upload(
                    surface_index,
                    GpuSurfaceRenderCanvasUploadClass::ImmutablePayload,
                ) else {
                    stats.mark_candidate_unavailable(
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    return true;
                };
                if byte_len != immutable_byte_len {
                    signal_plan_failure(
                        plan,
                        stats,
                        GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                    );
                    return true;
                }
            }
            let Some(byte_len) = plan.consume_upload(
                surface_index,
                GpuSurfaceRenderCanvasUploadClass::RendererParameter,
            ) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            if byte_len != renderer_parameter_byte_len {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            }
            plan.mark_execution_mutated();
            self.ensure_signal_buffer(super::resources::EnsureSignalBufferRequest {
                device: target.device,
                queue: target.queue,
                stats,
                key: surface.key,
                cache_key: buffer_cache_key,
                content_owner: RenderCanvasContentOwner::from_content(&surface.content),
                buckets: body.buckets,
                uniforms: &body.uniforms,
            });
            let Some(body_execution) = plan.consume_signal_body(surface_index, surface.key) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            let expected_body_operation =
                self.signal_body_operation(target.device, surface.key, body.body_key);
            if body_execution.device != super::wgpu_device_id(target.device)
                || body_execution.cache_key != body.body_key
                || body_execution.operation != expected_body_operation
            {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            }
            if matches!(
                body_execution.operation,
                GpuSurfaceRenderCanvasUploadSignalBodyOperation::Render
            ) {
                plan.mark_execution_mutated();
            }
            let Some(texture_view) = self.ensure_signal_body_texture(
                target.device,
                target.encoder,
                surface.key,
                body.body_key,
                stats,
            ) else {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            };
            self.render_texture_view_with_plan(
                target,
                TextureViewRenderRequest {
                    surface,
                    texture_identity: GpuSurfaceTextureIdentity::SignalBody(body.body_key),
                    texture_view: &texture_view,
                    source: [
                        0.0,
                        0.0,
                        body.body_key.width as f32,
                        body.body_key.height as f32,
                    ],
                    occlusion_regions,
                },
                Some((surface_index, plan)),
                stats,
            );
            return true;
        }
        self.ensure_pipeline(target.device, target.format);
        self.ensure_signal_pipeline(target.device, wgpu::TextureFormat::Rgba8Unorm);
        self.ensure_signal_buffer(super::resources::EnsureSignalBufferRequest {
            device: target.device,
            queue: target.queue,
            stats,
            key: surface.key,
            cache_key: buffer_cache_key,
            content_owner: RenderCanvasContentOwner::from_content(&surface.content),
            buckets: body.buckets,
            uniforms: &body.uniforms,
        });
        let Some(texture_view) = self.ensure_signal_body_texture(
            target.device,
            target.encoder,
            surface.key,
            body.body_key,
            stats,
        ) else {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return true;
        };
        self.render_texture_view(
            target,
            TextureViewRenderRequest {
                surface,
                texture_identity: GpuSurfaceTextureIdentity::SignalBody(body.body_key),
                texture_view: &texture_view,
                source: [
                    0.0,
                    0.0,
                    body.body_key.width as f32,
                    body.body_key.height as f32,
                ],
                occlusion_regions,
            },
            stats,
        );
        true
    }

    fn signal_render_source(
        &mut self,
        surface: &PaintGpuSurface,
        surface_index: usize,
        shape: GpuSignalRenderShape,
        upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<SignalRenderSource> {
        let summary = match &surface.content {
            GpuSurfaceContent::SignalBands { samples, .. } => {
                if let Some(plan) = upload_plan {
                    let Some(execution) = plan.consume_signal_summary(surface_index, surface.key)
                    else {
                        stats.mark_candidate_unavailable(
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        );
                        return None;
                    };
                    let source_identity = SignalSourceIdentity::from_content(&surface.content)
                        .expect("signal summary execution only receives signal content");
                    let expected_operation = self.signal_summary_cache_operation(
                        surface.key,
                        surface.revision,
                        source_identity,
                        shape.frames,
                        shape.band_count,
                        samples.len(),
                    );
                    if execution.revision != surface.revision
                        || execution.source_identity != source_identity
                        || execution.frames != shape.frames
                        || execution.band_count != shape.band_count
                        || execution.sample_count != samples.len()
                        || execution.operation != expected_operation
                    {
                        signal_plan_failure(
                            plan,
                            stats,
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        );
                        return None;
                    }
                    if matches!(
                        execution.operation,
                        GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Build
                    ) {
                        plan.mark_execution_mutated();
                    }
                }
                self.cached_signal_summary(super::resources::CachedSignalSummaryRequest {
                    key: surface.key,
                    revision: surface.revision,
                    source_identity: SignalSourceIdentity::from_content(&surface.content)
                        .expect("signal summary cache only receives signal content"),
                    frames: shape.frames,
                    band_count: shape.band_count,
                    samples,
                    stats,
                })
            }
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => Arc::clone(summary),
            _ => return None,
        };
        let sample_slide_frame_offset = signal_sample_slide_frame_offset(&surface.content);
        Some(SignalRenderSource {
            shape,
            summary,
            gain_preview: signal_gain_preview(&surface.content),
            sample_slide_frame_offset,
        })
    }

    fn validated_signal_render_shape_for_plan(
        &mut self,
        surface: &PaintGpuSurface,
        surface_index: usize,
        mut upload_plan: Option<&mut GpuSurfaceRenderCanvasUploadPlan>,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<GpuSignalRenderShape> {
        let expected = if let Some(plan) = upload_plan.as_deref_mut() {
            let Some(execution) = plan.consume_signal_validation(surface_index, surface.key) else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return None;
            };
            let (frames, band_count, summary) = signal_validation_dimensions(surface);
            let operation = signal_validation_operation(self, surface);
            if execution.frames != frames
                || execution.band_count != band_count
                || execution.summary != summary
                || execution.operation != operation
            {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return None;
            }
            if matches!(
                execution.operation,
                GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheUpdate
            ) {
                plan.mark_execution_mutated();
            }
            Some(execution)
        } else {
            None
        };
        let shape = match &surface.content {
            GpuSurfaceContent::SignalSummaryBands { .. } => {
                self.validated_signal_render_shape(surface, stats)
            }
            GpuSurfaceContent::SignalBands { .. } => surface.content.signal_render_shape(),
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => None,
        };
        if let Some(execution) = expected
            && execution.valid != shape.is_some()
        {
            if let Some(plan) = upload_plan {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            } else {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
            }
            return None;
        }
        shape
    }

    fn execute_composite_pipeline_for_signal(
        &mut self,
        target: &GpuSurfaceRenderTarget<'_>,
        execution: super::upload_plan::GpuSurfaceRenderCanvasUploadPipelineExecution,
    ) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
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
        if execution.device != device
            || execution.format != target.format
            || execution.generation != generation
            || execution.rebuild != rebuild
        {
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

    fn execute_signal_pipeline_for_signal(
        &mut self,
        target: &GpuSurfaceRenderTarget<'_>,
        execution: super::upload_plan::GpuSurfaceRenderCanvasUploadPipelineExecution,
    ) -> Result<(), GpuSurfaceRenderCanvasUploadPlanUnavailableReason> {
        let device = super::wgpu_device_id(target.device);
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let rebuild = self
            .signal_pipeline
            .as_ref()
            .is_none_or(|pipeline| !pipeline.matches_target(target.device, format));
        let generation = if rebuild {
            self.signal_pipeline_generation.wrapping_add(1)
        } else {
            self.signal_pipeline_generation
        };
        if execution.device != device
            || execution.format != format
            || execution.generation != generation
            || execution.rebuild != rebuild
        {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        self.ensure_signal_pipeline(target.device, format);
        if self.signal_pipeline_generation != execution.generation
            || self
                .signal_pipeline
                .as_ref()
                .is_none_or(|pipeline| !pipeline.matches_target(target.device, format))
        {
            return Err(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete);
        }
        Ok(())
    }

    pub(super) fn validated_signal_render_shape(
        &mut self,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<GpuSignalRenderShape> {
        let GpuSurfaceContent::SignalSummaryBands {
            frames,
            band_count,
            summary,
            ..
        } = &surface.content
        else {
            return surface.content.signal_render_shape();
        };
        let valid = if let Some(cached) = self
            .resources
            .signal_summary_validations
            .get(&surface.key)
            .filter(|cached| cached.matches(*frames, *band_count, summary))
        {
            stats.signal.summary_validation_cache_hits += 1;
            cached.valid
        } else {
            stats.signal.summary_validation_runs += 1;
            let valid = surface.content.signal_summary_payload_is_valid();
            self.resources.signal_summary_validations.insert(
                surface.key,
                CachedSignalSummaryValidation {
                    frames: *frames,
                    band_count: *band_count,
                    summary: Arc::clone(summary),
                    valid,
                },
            );
            valid
        };
        valid
            .then(|| {
                surface
                    .content
                    .signal_summary_render_shape_after_payload_validation()
            })
            .flatten()
    }
}

fn signal_plan_failure(
    plan: &mut GpuSurfaceRenderCanvasUploadPlan,
    stats: &mut GpuSurfaceRenderStats,
    reason: GpuSurfaceRenderCanvasUploadPlanUnavailableReason,
) {
    plan.veto_execution(reason);
    stats.mark_candidate_unavailable(reason);
}

fn signal_validation_dimensions(surface: &PaintGpuSurface) -> (usize, usize, usize) {
    match &surface.content {
        GpuSurfaceContent::SignalBands {
            frames, band_count, ..
        }
        | GpuSurfaceContent::SignalSummaryBands {
            frames, band_count, ..
        } => {
            let shape = surface.content.signal_render_shape();
            (
                shape.map_or(*frames, |shape| shape.frames),
                *band_count,
                match &surface.content {
                    GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                        Arc::as_ptr(summary) as *const () as usize
                    }
                    GpuSurfaceContent::SignalBands { .. } => 0,
                    GpuSurfaceContent::RgbaAtlas { .. }
                    | GpuSurfaceContent::CustomShader { .. } => 0,
                },
            )
        }
        GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => (0, 0, 0),
    }
}

fn signal_validation_operation(
    renderer: &GpuSurfaceRenderer,
    surface: &PaintGpuSurface,
) -> GpuSurfaceRenderCanvasUploadSignalValidationOperation {
    let GpuSurfaceContent::SignalSummaryBands {
        frames,
        band_count,
        summary,
        ..
    } = &surface.content
    else {
        return GpuSurfaceRenderCanvasUploadSignalValidationOperation::Pure;
    };
    if renderer
        .resources
        .signal_summary_validations
        .get(&surface.key)
        .is_some_and(|cached| cached.matches(*frames, *band_count, summary))
    {
        GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheHit
    } else {
        GpuSurfaceRenderCanvasUploadSignalValidationOperation::CacheUpdate
    }
}

fn signal_body_request<'a>(
    target: &GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SignalBodyRequest<'a>> {
    signal_body_request_at_dpi(surface, source, target.dpi_scale)
}

fn signal_body_request_at_dpi<'a>(
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
    dpi_scale: DpiScale,
) -> Option<SignalBodyRequest<'a>> {
    let selected = selected_signal_level(dpi_scale, surface, source)?;
    let body_key = signal_body_cache_key(SignalBodyKeyRequest {
        surface,
        source,
        selected: &selected,
        dpi_scale,
    })?;
    let uniforms = signal_uniforms(source, &selected, body_key);
    Some(SignalBodyRequest {
        body_key,
        level_index: selected.index,
        bucket_start: selected.bucket_window.start,
        bucket_count: selected.bucket_window.bucket_count(),
        buckets: selected
            .bucket_window
            .buckets(selected.level, source.shape.band_count),
        uniforms,
    })
}

fn selected_signal_level<'a>(
    dpi_scale: DpiScale,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SelectedSignalLevel<'a>> {
    let visible = (source.shape.frame_range[1] - source.shape.frame_range[0]).max(1.0);
    let physical_width = dpi_scale.logical_to_physical(surface.rect.width()).max(1.0);
    let index = source
        .summary
        .level_for_frames_per_pixel(visible / physical_width);
    let level = source.summary.levels.get(index)?;
    let bucket_window = signal_bucket_window(
        signal_bucket_frame_range(source),
        level,
        source.shape.band_count,
    )?;
    Some(SelectedSignalLevel {
        index,
        level,
        bucket_window,
    })
}

fn signal_body_cache_key(request: SignalBodyKeyRequest<'_>) -> Option<SignalBodyCacheKey> {
    let extent = surface_pixel_extent(request.surface.rect, request.dpi_scale)?;
    Some(SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
        revision: request.surface.revision,
        content_identity: RenderCanvasContentIdentity::from_content(&request.surface.content),
        extent,
        frames: request.source.shape.frames,
        band_count: request.source.shape.band_count,
        frame_range: request.source.shape.frame_range,
        sample_slide_frame_offset: request.source.sample_slide_frame_offset,
        sample_count: request
            .selected
            .bucket_window
            .sample_count(request.source.shape.band_count),
        level_index: request.selected.index,
        gain_preview: request.source.gain_preview,
    }))
}

fn signal_bucket_frame_range(source: &SignalRenderSource) -> [f32; 2] {
    if source.sample_slide_frame_offset == 0 {
        return source.shape.frame_range;
    }
    let frames = source.shape.frames as f32;
    if frames <= 1.0 {
        return source.shape.frame_range;
    }
    let start = source.shape.frame_range[0] - source.sample_slide_frame_offset as f32;
    let end = source.shape.frame_range[1] - source.sample_slide_frame_offset as f32;
    if start >= 0.0 && end <= frames {
        [start, end]
    } else {
        [0.0, frames]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Vector2},
        runtime::{GpuSignalSummaryBucket, GpuSignalSummaryLevel, GpuSurfaceCapabilities},
    };

    fn summary_surface(
        key: u64,
        frames: usize,
        band_count: usize,
        summary: Arc<GpuSignalSummary>,
    ) -> PaintGpuSurface {
        PaintGpuSurface {
            widget_id: 1,
            key,
            revision: 1,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 200.0)),
            content: GpuSurfaceContent::SignalSummaryBands {
                frames,
                band_count,
                frame_range: [0.0, frames as f32],
                summary,
                gain_preview: None,
                sample_slide_frame_offset: 0,
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        }
    }

    #[test]
    fn retained_summary_validation_runs_once_per_summary_identity() {
        let samples = vec![0.25; 16_384];
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            &samples,
            samples.len(),
            1,
        ));
        let surface = summary_surface(7, samples.len(), 1, Arc::clone(&summary));
        let mut renderer = GpuSurfaceRenderer::default();
        let mut stats = GpuSurfaceRenderStats::default();

        assert!(
            renderer
                .validated_signal_render_shape(&surface, &mut stats)
                .is_some()
        );
        assert!(
            renderer
                .validated_signal_render_shape(&surface, &mut stats)
                .is_some()
        );

        assert_eq!(stats.signal.summary_validation_runs, 1);
        assert_eq!(stats.signal.summary_validation_cache_hits, 1);

        let replacement = summary_surface(7, samples.len(), 1, Arc::new((*summary).clone()));
        assert!(
            renderer
                .validated_signal_render_shape(&replacement, &mut stats)
                .is_some()
        );
        assert_eq!(stats.signal.summary_validation_runs, 2);
    }

    #[test]
    fn retained_summary_validation_rejects_and_caches_malformed_payloads() {
        let malformed = Arc::new(GpuSignalSummary {
            frames: 1,
            band_count: 1,
            levels: vec![GpuSignalSummaryLevel {
                bucket_frames: 1,
                buckets: Arc::from([GpuSignalSummaryBucket {
                    min: f32::NAN,
                    max: 1.0,
                }]),
            }],
        });
        let surface = summary_surface(9, 1, 1, malformed);
        let mut renderer = GpuSurfaceRenderer::default();
        let mut stats = GpuSurfaceRenderStats::default();

        assert_eq!(
            renderer.validated_signal_render_shape(&surface, &mut stats),
            None
        );
        assert_eq!(
            renderer.validated_signal_render_shape(&surface, &mut stats),
            None
        );

        assert_eq!(stats.signal.summary_validation_runs, 1);
        assert_eq!(stats.signal.summary_validation_cache_hits, 1);
    }

    #[test]
    fn retained_summary_validation_rechecks_declared_shape_changes() {
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            &[0.0, 0.5, -0.5, 1.0],
            4,
            1,
        ));
        let valid = summary_surface(11, 4, 1, Arc::clone(&summary));
        let changed_shape = summary_surface(11, 5, 1, summary);
        let mut renderer = GpuSurfaceRenderer::default();
        let mut stats = GpuSurfaceRenderStats::default();

        assert!(
            renderer
                .validated_signal_render_shape(&valid, &mut stats)
                .is_some()
        );
        assert_eq!(
            renderer.validated_signal_render_shape(&changed_shape, &mut stats),
            None
        );

        assert_eq!(stats.signal.summary_validation_runs, 2);
        assert_eq!(stats.signal.summary_validation_cache_hits, 0);
    }
}
