use super::super::signal_summary_prepare::PreparedSummary;
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
    prepared: Option<PreparedSummary>,
    shape: GpuSignalRenderShape,
    data: SignalRenderData,
    gain_preview: Option<GpuSignalGainPreview>,
    sample_slide_frame_offset: i64,
}

enum SignalRenderData {
    Overview(Arc<GpuSignalSummary>),
    Tile(SignalTileData),
}

struct SignalTileData {
    buckets: Arc<[GpuSignalSummaryBucket]>,
    bucket_frames: usize,
    band_count: usize,
    query_start_bucket: f32,
    query_span_buckets: f32,
}

#[derive(Clone, Copy)]
struct TileQueryMapping {
    start_bucket: f32,
    span_buckets: f32,
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

enum SelectedSignalData<'a> {
    Overview(SelectedSignalLevel<'a>),
    Tile(&'a SignalTileData),
}

struct SignalBodyKeyRequest<'a> {
    surface: &'a PaintGpuSurface,
    source: &'a SignalRenderSource,
    selected: &'a SelectedSignalData<'a>,
    dpi_scale: DpiScale,
}

#[derive(Clone)]
struct SignalSummaryPreflightIdentity {
    revision: u64,
    source_identity: SignalSourceIdentity,
    frames: usize,
    band_count: usize,
    sample_count: usize,
    prepared: PreparedSummary,
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
    pub(super) fn release_prepared_summaries(&mut self) {
        self.summaries.clear();
    }

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
    ) -> Option<PreparedSummary> {
        let GpuSurfaceContent::SignalBands {
            samples,
            frames,
            band_count,
            ..
        } = &surface.content
        else {
            return None;
        };
        let source_identity = SignalSourceIdentity::samples(samples, *frames, *band_count);
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
                    prepared: cached.prepared.clone(),
                })
        })?;
        if cached.revision != surface.revision
            || cached.source_identity != source_identity
            || cached.frames != shape.frames
            || cached.band_count != shape.band_count
            || cached.sample_count != samples.len()
            || !cached
                .prepared
                .matches_raw_surface(&surface.content, surface.revision)
        {
            return None;
        }
        let prepared = cached.prepared.clone();
        self.summaries.insert(surface.key, cached);
        Some(prepared)
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

        let (summary, prepared) = match &surface.content {
            GpuSurfaceContent::SignalBands { .. } => {
                let Some(prepared) = signal_state.signal_summary(self, surface, shape) else {
                    return SignalUploadPreflight {
                        renderable: false,
                        unavailable: Some(
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        ),
                    };
                };
                (Arc::clone(prepared.summary()), Some(prepared))
            }
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => (Arc::clone(summary), None),
            _ => {
                return SignalUploadPreflight {
                    renderable: false,
                    unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
                };
            }
        };
        if let GpuSurfaceContent::SignalBands {
            samples,
            frames,
            band_count,
            ..
        } = &surface.content
        {
            actions.push(GpuSurfaceRenderCanvasUploadAction::SignalSummary {
                surface_index,
                key: surface.key,
                revision: surface.revision,
                source_identity: SignalSourceIdentity::samples(samples, *frames, *band_count),
                frames: shape.frames,
                band_count: shape.band_count,
                sample_count: samples.len(),
                operation: GpuSurfaceRenderCanvasUploadSignalSummaryOperation::Reuse,
            });
        }
        let sample_slide_frame_offset = signal_sample_slide_frame_offset(&surface.content);
        let data = prepared
            .as_ref()
            .and_then(|prepared| {
                prepared.tile().and_then(|tile| {
                    tile_query_mapping(shape, sample_slide_frame_offset, tile).map(|query| {
                        SignalRenderData::Tile(SignalTileData {
                            buckets: Arc::clone(&tile.buckets),
                            bucket_frames: tile.bucket_frames,
                            band_count: tile.band_count,
                            query_start_bucket: query.start_bucket,
                            query_span_buckets: query.span_buckets,
                        })
                    })
                })
            })
            .unwrap_or(SignalRenderData::Overview(summary));
        let source = SignalRenderSource {
            prepared,
            shape,
            data,
            gain_preview: signal_gain_preview(&surface.content),
            sample_slide_frame_offset,
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
        let source_identity = match &surface.content {
            GpuSurfaceContent::SignalBands {
                samples,
                frames,
                band_count,
                ..
            } => SignalSourceIdentity::samples(samples, *frames, *band_count),
            GpuSurfaceContent::SignalSummaryBands {
                summary,
                frames,
                band_count,
                ..
            } => SignalSourceIdentity::summary(summary, *frames, *band_count),
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => {
                return SignalUploadPreflight {
                    renderable: false,
                    unavailable: Some(GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid),
                };
            }
        };
        let buffer_cache_key = SignalBufferCacheKey::new(
            surface.revision,
            source_identity,
            body.level_index,
            body.bucket_start,
            body.bucket_count,
            source.prepared.as_ref().map(PreparedSummary::asset_key),
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
        let source_identity = match &surface.content {
            GpuSurfaceContent::SignalBands {
                samples,
                frames,
                band_count,
                ..
            } => SignalSourceIdentity::samples(samples, *frames, *band_count),
            GpuSurfaceContent::SignalSummaryBands {
                summary,
                frames,
                band_count,
                ..
            } => SignalSourceIdentity::summary(summary, *frames, *band_count),
            GpuSurfaceContent::RgbaAtlas { .. } | GpuSurfaceContent::CustomShader { .. } => {
                stats.mark_candidate_unavailable(
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Invalid,
                );
                return true;
            }
        };
        let buffer_cache_key = SignalBufferCacheKey::new(
            surface.revision,
            source_identity,
            body.level_index,
            body.bucket_start,
            body.bucket_count,
            source.prepared.as_ref().map(PreparedSummary::asset_key),
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
            if !self.ensure_signal_buffer(super::resources::EnsureSignalBufferRequest {
                device: target.device,
                queue: target.queue,
                stats,
                key: surface.key,
                cache_key: buffer_cache_key,
                content_owner: source.prepared.as_ref().map_or_else(
                    || RenderCanvasContentOwner::from_content(&surface.content),
                    |prepared| RenderCanvasContentOwner::PreparedSignal(prepared.clone()),
                ),
                buckets: body.buckets,
                uniforms: &body.uniforms,
                gpu_budget: source
                    .prepared
                    .as_ref()
                    .map(|prepared| Arc::clone(prepared.gpu_budget())),
            }) {
                signal_plan_failure(
                    plan,
                    stats,
                    GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                );
                return true;
            }
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
                source.prepared.as_ref().map(PreparedSummary::gpu_budget),
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
        if !self.ensure_signal_buffer(super::resources::EnsureSignalBufferRequest {
            device: target.device,
            queue: target.queue,
            stats,
            key: surface.key,
            cache_key: buffer_cache_key,
            content_owner: source.prepared.as_ref().map_or_else(
                || RenderCanvasContentOwner::from_content(&surface.content),
                |prepared| RenderCanvasContentOwner::PreparedSignal(prepared.clone()),
            ),
            buckets: body.buckets,
            uniforms: &body.uniforms,
            gpu_budget: source
                .prepared
                .as_ref()
                .map(|prepared| Arc::clone(prepared.gpu_budget())),
        }) {
            stats.mark_candidate_unavailable(
                GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
            );
            return true;
        }
        let Some(texture_view) = self.ensure_signal_body_texture(
            target.device,
            target.encoder,
            surface.key,
            body.body_key,
            stats,
            source.prepared.as_ref().map(PreparedSummary::gpu_budget),
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
        let mut prepared = None;
        let overview = match &surface.content {
            GpuSurfaceContent::SignalBands {
                samples,
                frames,
                band_count,
                ..
            } => {
                if let Some(plan) = upload_plan {
                    let Some(execution) = plan.consume_signal_summary(surface_index, surface.key)
                    else {
                        stats.mark_candidate_unavailable(
                            GpuSurfaceRenderCanvasUploadPlanUnavailableReason::Incomplete,
                        );
                        return None;
                    };
                    let source_identity =
                        SignalSourceIdentity::samples(samples, *frames, *band_count);
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
                let ready =
                    self.cached_signal_summary(super::resources::CachedSignalSummaryRequest {
                        key: surface.key,
                        revision: surface.revision,
                        source_identity: SignalSourceIdentity::samples(
                            samples,
                            *frames,
                            *band_count,
                        ),
                        frames: shape.frames,
                        band_count: shape.band_count,
                        samples,
                        stats,
                    })?;
                let summary = Arc::clone(ready.summary());
                prepared = Some(ready);
                summary
            }
            GpuSurfaceContent::SignalSummaryBands { summary, .. } => Arc::clone(summary),
            _ => return None,
        };
        let sample_slide_frame_offset = signal_sample_slide_frame_offset(&surface.content);
        let data = prepared
            .as_ref()
            .and_then(|prepared| {
                prepared.tile().and_then(|tile| {
                    tile_query_mapping(shape, sample_slide_frame_offset, tile).map(|query| {
                        SignalRenderData::Tile(SignalTileData {
                            buckets: Arc::clone(&tile.buckets),
                            bucket_frames: tile.bucket_frames,
                            band_count: tile.band_count,
                            query_start_bucket: query.start_bucket,
                            query_span_buckets: query.span_buckets,
                        })
                    })
                })
            })
            .unwrap_or(SignalRenderData::Overview(overview));
        Some(SignalRenderSource {
            prepared,
            shape,
            data,
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
    match selected {
        SelectedSignalData::Overview(selected) => Some(SignalBodyRequest {
            body_key,
            level_index: selected.index,
            bucket_start: selected.bucket_window.start,
            bucket_count: selected.bucket_window.bucket_count(),
            buckets: selected
                .bucket_window
                .buckets(selected.level, source.shape.band_count),
            uniforms,
        }),
        SelectedSignalData::Tile(tile) => Some(SignalBodyRequest {
            body_key,
            level_index: usize::MAX,
            bucket_start: 0,
            bucket_count: tile.buckets.len() / tile.band_count.max(1),
            buckets: &tile.buckets,
            uniforms,
        }),
    }
}

fn selected_signal_level<'a>(
    dpi_scale: DpiScale,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SelectedSignalData<'a>> {
    if let SignalRenderData::Tile(tile) = &source.data {
        return Some(SelectedSignalData::Tile(tile));
    }
    let SignalRenderData::Overview(summary) = &source.data else {
        return None;
    };
    let visible = (source.shape.frame_range[1] - source.shape.frame_range[0]).max(1.0);
    let physical_width = dpi_scale.logical_to_physical(surface.rect.width()).max(1.0);
    let index = summary.level_for_frames_per_pixel(visible / physical_width);
    let level = summary.levels.get(index)?;
    let bucket_window = signal_bucket_window(
        signal_bucket_frame_range(source),
        level,
        source.shape.band_count,
    )?;
    Some(SelectedSignalData::Overview(SelectedSignalLevel {
        index,
        level,
        bucket_window,
    }))
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
        sample_count: match request.selected {
            SelectedSignalData::Overview(selected) => selected
                .bucket_window
                .sample_count(request.source.shape.band_count),
            SelectedSignalData::Tile(tile) => tile.buckets.len(),
        },
        level_index: match request.selected {
            SelectedSignalData::Overview(selected) => selected.index,
            SelectedSignalData::Tile(_) => usize::MAX,
        },
        gain_preview: request.source.gain_preview,
        prepared_asset: request
            .source
            .prepared
            .as_ref()
            .map(PreparedSummary::asset_key),
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

fn tile_query_mapping(
    shape: GpuSignalRenderShape,
    slide: i64,
    tile: &crate::runtime::BoundedSignalTile,
) -> Option<TileQueryMapping> {
    if tile.source_frames != shape.frames
        || tile.band_count != shape.band_count
        || tile.bucket_frames == 0
        || tile.buckets.len() % tile.band_count.max(1) != 0
    {
        return None;
    }
    let start = f64::from(shape.frame_range[0]);
    let end = f64::from(shape.frame_range[1]);
    let source_frames = tile.source_frames as f64;
    if !start.is_finite() || !end.is_finite() || end <= start || source_frames <= 0.0 {
        return None;
    }
    let visible = end - start;
    // Resolve the integer slide before converting to a local float. Large
    // signed slides must not lose their remainder through an f64 conversion.
    let integral_start = start.floor();
    let physical_start = ((integral_start as i128 - i128::from(slide))
        .rem_euclid(tile.source_frames as i128)) as f64
        + (start - integral_start);
    let tile_start = tile.first_frame as f64;
    let tile_span =
        (tile.buckets.len() / tile.band_count.max(1)).checked_mul(tile.bucket_frames)?;
    let tile_end = tile_start + tile_span as f64;
    let cycles = ((tile_start - physical_start) / source_frames)
        .ceil()
        .max(0.0);
    let query_start = physical_start + cycles * source_frames;
    if query_start < tile_start || query_start + visible > tile_end {
        return None;
    }
    let bucket_frames = tile.bucket_frames as f64;
    Some(TileQueryMapping {
        start_bucket: ((query_start - tile_start) / bucket_frames) as f32,
        span_buckets: (visible / bucket_frames) as f32,
    })
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

    fn signal_preflight_actions(
        renderer: &GpuSurfaceRenderer,
        state: &mut SignalUploadPreflightState,
        surface: &PaintGpuSurface,
    ) -> Vec<GpuSurfaceRenderCanvasUploadAction> {
        let mut composite_state = AtlasUploadPreflightState::default();
        let mut actions = Vec::new();
        let result = renderer.preflight_signal_upload_actions(
            GpuSurfaceRenderCanvasUploadTarget::new(7, wgpu::TextureFormat::Bgra8Unorm, 640, 200),
            DpiScale::ONE,
            0,
            surface,
            SignalUploadPreflightContext {
                composite_state: &mut composite_state,
                signal_state: state,
                actions: &mut actions,
            },
        );
        assert!(result.renderable);
        assert_eq!(result.unavailable, None);
        actions
    }

    fn signal_buffer_action(
        actions: &[GpuSurfaceRenderCanvasUploadAction],
    ) -> (
        SignalBufferCacheKey,
        GpuSurfaceRenderCanvasUploadSignalBufferOperation,
    ) {
        actions
            .iter()
            .find_map(|action| match action {
                GpuSurfaceRenderCanvasUploadAction::SignalBuffer {
                    cache_key,
                    operation,
                    ..
                } => Some((*cache_key, *operation)),
                _ => None,
            })
            .expect("signal buffer action")
    }

    fn signal_body_operation(
        actions: &[GpuSurfaceRenderCanvasUploadAction],
    ) -> GpuSurfaceRenderCanvasUploadSignalBodyOperation {
        actions
            .iter()
            .find_map(|action| match action {
                GpuSurfaceRenderCanvasUploadAction::SignalBody { operation, .. } => {
                    Some(*operation)
                }
                _ => None,
            })
            .expect("signal body action")
    }

    fn uploads_of_class(
        actions: &[GpuSurfaceRenderCanvasUploadAction],
        class: GpuSurfaceRenderCanvasUploadClass,
    ) -> usize {
        actions
            .iter()
            .filter(|action| {
                matches!(
                    action,
                    GpuSurfaceRenderCanvasUploadAction::Upload {
                        class: action_class,
                        ..
                    } if *action_class == class
                )
            })
            .count()
    }

    #[test]
    fn signal_preflight_reuses_bucket_data_for_nearby_presentation_updates() {
        let samples = vec![0.25; 4_096];
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            &samples, 4_096, 1,
        ));
        let first = summary_surface(7, 4_096, 1, Arc::clone(&summary));
        let mut presented = first.clone();
        let GpuSurfaceContent::SignalSummaryBands {
            frame_range,
            gain_preview,
            ..
        } = &mut presented.content
        else {
            unreachable!()
        };
        *frame_range = [200.4, 403.4];
        *gain_preview = Some(GpuSignalGainPreview {
            start: 200.4,
            end: 403.4,
            gain: 0.75,
            fade_in_length: 4.0,
            fade_in_curve: 0.5,
            fade_in_mute: 0.0,
            fade_in_outer_gain: 1.0,
            fade_out_length: 4.0,
            fade_out_curve: 0.5,
            fade_out_mute: 0.0,
            fade_out_outer_gain: 1.0,
        });
        let mut first = first;
        let GpuSurfaceContent::SignalSummaryBands { frame_range, .. } = &mut first.content else {
            unreachable!()
        };
        *frame_range = [200.2, 403.2];

        let renderer = GpuSurfaceRenderer::default();
        let mut state = SignalUploadPreflightState::default();
        let initial_actions = signal_preflight_actions(&renderer, &mut state, &first);
        let presented_actions = signal_preflight_actions(&renderer, &mut state, &presented);

        assert_eq!(
            signal_buffer_action(&initial_actions).1,
            GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload
        );
        assert_eq!(
            signal_buffer_action(&presented_actions).1,
            GpuSurfaceRenderCanvasUploadSignalBufferOperation::Reuse
        );
        assert_eq!(
            signal_body_operation(&presented_actions),
            GpuSurfaceRenderCanvasUploadSignalBodyOperation::Render
        );
        assert_eq!(
            uploads_of_class(
                &presented_actions,
                GpuSurfaceRenderCanvasUploadClass::ImmutablePayload
            ),
            0
        );
        assert_eq!(
            uploads_of_class(
                &presented_actions,
                GpuSurfaceRenderCanvasUploadClass::RendererParameter
            ),
            2
        );
    }

    #[test]
    fn bounded_tile_query_maps_wrapped_slide_to_contiguous_local_buckets() {
        let tile = crate::runtime::BoundedSignalTile {
            first_frame: 88,
            source_frames: 100,
            band_count: 1,
            bucket_frames: 4,
            buckets: Arc::from([GpuSignalSummaryBucket::default(); 8]),
        };
        let shape = GpuSignalRenderShape {
            frames: 100,
            band_count: 1,
            frame_range: [96.0, 112.0],
            sample_count: 100,
        };

        let query = tile_query_mapping(shape, 0, &tile).expect("wrapped tile covers viewport");

        assert_eq!(query.start_bucket, 2.0);
        assert_eq!(query.span_buckets, 4.0);
    }

    #[test]
    fn bounded_tile_query_preserves_extreme_integer_slide_remainder() {
        let tile = crate::runtime::BoundedSignalTile {
            first_frame: 0,
            source_frames: 100,
            band_count: 1,
            bucket_frames: 1,
            buckets: Arc::from([GpuSignalSummaryBucket::default(); 200]),
        };
        let shape = GpuSignalRenderShape {
            frames: 100,
            band_count: 1,
            frame_range: [10.25, 20.25],
            sample_count: 100,
        };
        for slide in [i64::MIN, i64::MAX] {
            let query = tile_query_mapping(shape, slide, &tile).unwrap();
            let expected = (10_i128 - i128::from(slide)).rem_euclid(100) as f32 + 0.25;
            assert_eq!(query.start_bucket, expected);
            assert_eq!(query.span_buckets, 10.0);
        }
    }

    #[test]
    fn bounded_tile_query_refuses_a_tile_that_does_not_cover_slide_range() {
        let tile = crate::runtime::BoundedSignalTile {
            first_frame: 8,
            source_frames: 100,
            band_count: 1,
            bucket_frames: 4,
            buckets: Arc::from([GpuSignalSummaryBucket::default(); 2]),
        };
        let shape = GpuSignalRenderShape {
            frames: 100,
            band_count: 1,
            frame_range: [40.0, 56.0],
            sample_count: 100,
        };

        assert!(tile_query_mapping(shape, 0, &tile).is_none());
    }

    #[test]
    fn signal_preflight_reuploads_bucket_data_when_window_or_detail_changes() {
        let samples = vec![0.25; 4_096];
        let summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
            &samples, 4_096, 1,
        ));
        let mut first = summary_surface(7, 4_096, 1, Arc::clone(&summary));
        let GpuSurfaceContent::SignalSummaryBands { frame_range, .. } = &mut first.content else {
            unreachable!()
        };
        *frame_range = [200.2, 403.2];
        let mut crossed_window = first.clone();
        let GpuSurfaceContent::SignalSummaryBands { frame_range, .. } = &mut crossed_window.content
        else {
            unreachable!()
        };
        *frame_range = [600.2, 803.2];
        let mut changed_detail = first.clone();
        changed_detail.rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(64.0, 200.0));

        let renderer = GpuSurfaceRenderer::default();
        let mut state = SignalUploadPreflightState::default();
        let first_actions = signal_preflight_actions(&renderer, &mut state, &first);
        let window_actions = signal_preflight_actions(&renderer, &mut state, &crossed_window);
        let detail_actions = signal_preflight_actions(&renderer, &mut state, &changed_detail);

        let first_key = signal_buffer_action(&first_actions).0;
        let window_key = signal_buffer_action(&window_actions).0;
        let detail_key = signal_buffer_action(&detail_actions).0;
        assert_ne!(first_key.bucket_start, window_key.bucket_start);
        assert_eq!(
            signal_buffer_action(&window_actions).1,
            GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload
        );
        assert_ne!(first_key.level_index, detail_key.level_index);
        assert_eq!(
            signal_buffer_action(&detail_actions).1,
            GpuSurfaceRenderCanvasUploadSignalBufferOperation::Upload
        );
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
