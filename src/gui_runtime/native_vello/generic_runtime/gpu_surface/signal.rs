use super::atlas::TextureViewRenderRequest;
use super::gpu_surface_types::{
    CachedSignalSummaryValidation, GpuSurfaceTextureIdentity, SignalBodyCacheKey,
    SignalBodyCacheKeyParts, SignalBufferCacheKey, SignalUniforms,
};
use super::passes::surface_pixel_extent;
use super::stats::GpuSurfaceRenderStats;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
#[path = "signal/uniforms.rs"]
mod uniforms;
#[path = "signal/window.rs"]
mod window;
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryBucket,
    GpuSignalSummaryLevel, GpuSurfaceContent, PaintGpuSurface,
};
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
    target: &'a GpuSurfaceRenderTarget<'a>,
}

impl GpuSurfaceRenderer {
    pub(super) fn render_signal(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        shape: GpuSignalRenderShape,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(source) = self.signal_render_source(surface, shape, stats) else {
            return;
        };
        let Some(body) = signal_body_request(target, surface, &source) else {
            return;
        };
        self.ensure_pipeline(target.device, target.format);
        self.ensure_signal_pipeline(target.device, wgpu::TextureFormat::Rgba8Unorm);
        self.ensure_signal_buffer(
            target.device,
            target.queue,
            surface.key,
            SignalBufferCacheKey::new(
                surface.revision,
                body.level_index,
                body.bucket_start,
                body.bucket_count,
            ),
            body.buckets,
            &body.uniforms,
        );
        let Some(texture_view) = self.ensure_signal_body_texture(
            target.device,
            target.encoder,
            surface.key,
            body.body_key,
            stats,
        ) else {
            return;
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
    }

    fn signal_render_source(
        &mut self,
        surface: &PaintGpuSurface,
        shape: GpuSignalRenderShape,
        stats: &mut GpuSurfaceRenderStats,
    ) -> Option<SignalRenderSource> {
        let summary = match &surface.content {
            GpuSurfaceContent::SignalBands { samples, .. } => self.cached_signal_summary(
                surface.key,
                surface.revision,
                shape.frames,
                shape.band_count,
                samples,
                stats,
            ),
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

fn signal_body_request<'a>(
    target: &GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SignalBodyRequest<'a>> {
    let selected = selected_signal_level(target, surface, source)?;
    let body_key = signal_body_cache_key(SignalBodyKeyRequest {
        surface,
        source,
        selected: &selected,
        target,
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
    target: &GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SelectedSignalLevel<'a>> {
    let visible = (source.shape.frame_range[1] - source.shape.frame_range[0]).max(1.0);
    let physical_width = target
        .dpi_scale
        .logical_to_physical(surface.rect.width())
        .max(1.0);
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
    let extent = surface_pixel_extent(request.surface.rect, request.target.dpi_scale)?;
    Some(SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
        revision: request.surface.revision,
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
