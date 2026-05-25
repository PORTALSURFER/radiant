use super::atlas::TextureViewRenderRequest;
use super::gpu_surface_types::{
    GpuSurfaceTextureIdentity, SignalBodyCacheKey, SignalBodyCacheKeyParts, SignalBufferCacheKey,
    SignalUniforms,
};
use super::passes::surface_pixel_extent;
use super::stats::GpuSurfaceRenderStats;
use super::{GpuSurfaceRenderTarget, GpuSurfaceRenderer};
use crate::gui::types::Rect as UiRect;
use crate::runtime::{
    GpuSignalGainPreview, GpuSignalRenderShape, GpuSignalSummary, GpuSignalSummaryLevel,
    GpuSurfaceContent, PaintGpuSurface,
};
use std::sync::Arc;
use vello::wgpu;

struct SignalRenderSource {
    shape: GpuSignalRenderShape,
    summary: Arc<GpuSignalSummary>,
    gain_preview: Option<GpuSignalGainPreview>,
}

struct SignalBodyRequest<'a> {
    body_key: SignalBodyCacheKey,
    level_index: usize,
    level: &'a GpuSignalSummaryLevel,
    uniforms: SignalUniforms,
}

struct SelectedSignalLevel<'a> {
    index: usize,
    level: &'a GpuSignalSummaryLevel,
}

impl GpuSurfaceRenderer {
    pub(super) fn render_signal(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        surface: &PaintGpuSurface,
        occlusion_regions: &[UiRect],
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let Some(shape) = surface.content.signal_render_shape() else {
            return;
        };
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
            SignalBufferCacheKey::new(surface.revision, body.level_index),
            body.level.buckets.as_ref(),
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
        Some(SignalRenderSource {
            shape,
            summary,
            gain_preview: signal_gain_preview(&surface.content),
        })
    }
}

fn signal_body_request<'a>(
    target: &GpuSurfaceRenderTarget<'_>,
    surface: &PaintGpuSurface,
    source: &'a SignalRenderSource,
) -> Option<SignalBodyRequest<'a>> {
    let selected = selected_signal_level(target, surface, source)?;
    let body_key = signal_body_cache_key(surface, source, &selected, target)?;
    let uniforms = signal_uniforms(source, &selected, body_key);
    Some(SignalBodyRequest {
        body_key,
        level_index: selected.index,
        level: selected.level,
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
    Some(SelectedSignalLevel {
        index,
        level: source.summary.levels.get(index)?,
    })
}

fn signal_body_cache_key(
    surface: &PaintGpuSurface,
    source: &SignalRenderSource,
    selected: &SelectedSignalLevel<'_>,
    target: &GpuSurfaceRenderTarget<'_>,
) -> Option<SignalBodyCacheKey> {
    let extent = surface_pixel_extent(surface.rect, target.dpi_scale)?;
    Some(SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
        revision: surface.revision,
        extent,
        frames: source.shape.frames,
        band_count: source.shape.band_count,
        frame_range: source.shape.frame_range,
        sample_count: selected.level.buckets.len(),
        level_index: selected.index,
        gain_preview: source.gain_preview,
    }))
}

fn signal_uniforms(
    source: &SignalRenderSource,
    selected: &SelectedSignalLevel<'_>,
    body_key: SignalBodyCacheKey,
) -> SignalUniforms {
    let gain_uniforms = signal_gain_preview_uniforms(source.gain_preview);
    SignalUniforms {
        dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
        frame_range: [
            source.shape.frame_range[0],
            source.shape.frame_range[1],
            source.shape.frames as f32,
            source.shape.band_count as f32,
        ],
        summary_meta: [
            selected.level.bucket_frames as f32,
            (selected.level.buckets.len() / source.shape.band_count) as f32,
            selected.index as f32,
            0.0,
        ],
        gain_preview_a: gain_uniforms[0],
        gain_preview_b: gain_uniforms[1],
        gain_preview_c: gain_uniforms[2],
        target_size: [body_key.width as f32, body_key.height as f32],
        cursor_ratio: -1.0,
        cursor_width: 1.0,
        cursor_color: [1.0, 1.0, 1.0, 0.92],
    }
}

fn signal_gain_preview(content: &GpuSurfaceContent) -> Option<GpuSignalGainPreview> {
    match content {
        GpuSurfaceContent::SignalSummaryBands { gain_preview, .. } => *gain_preview,
        _ => None,
    }
}

fn signal_gain_preview_uniforms(preview: Option<GpuSignalGainPreview>) -> [[f32; 4]; 3] {
    let Some(preview) = preview else {
        return [[0.0; 4]; 3];
    };
    [
        [1.0, preview.start, preview.end, preview.gain],
        [
            preview.fade_in_length,
            preview.fade_in_curve,
            preview.fade_out_length,
            preview.fade_out_curve,
        ],
        [preview.fade_in_mute, preview.fade_out_mute, 0.0, 0.0],
    ]
}

#[cfg(test)]
#[path = "signal/tests.rs"]
mod tests;
