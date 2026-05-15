use super::atlas::TextureViewRenderRequest;
use super::*;
use crate::runtime::{GpuSurfaceContent, PaintGpuSurface};
use std::sync::Arc;

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
            _ => return,
        };
        let visible = (shape.frame_range[1] - shape.frame_range[0]).max(1.0);
        let frames_per_pixel = visible / surface.rect.width().max(1.0);
        let level_index = summary.level_for_frames_per_pixel(frames_per_pixel);
        let Some(level) = summary.levels.get(level_index) else {
            return;
        };
        let Some(body_extent) = surface_pixel_extent(surface.rect) else {
            return;
        };
        let gain_preview = signal_gain_preview(&surface.content);
        let body_key = SignalBodyCacheKey::new(SignalBodyCacheKeyParts {
            revision: surface.revision,
            extent: body_extent,
            frames: shape.frames,
            band_count: shape.band_count,
            frame_range: shape.frame_range,
            sample_count: level.buckets.len(),
            level_index,
            gain_preview,
        });
        self.ensure_pipeline(target.device, target.format);
        self.ensure_signal_pipeline(target.device, wgpu::TextureFormat::Rgba8Unorm);
        let gain_uniforms = signal_gain_preview_uniforms(gain_preview);
        let uniforms = SignalUniforms {
            dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            frame_range: [
                shape.frame_range[0],
                shape.frame_range[1],
                shape.frames as f32,
                shape.band_count as f32,
            ],
            summary_meta: [
                level.bucket_frames as f32,
                (level.buckets.len() / shape.band_count) as f32,
                level_index as f32,
                0.0,
            ],
            gain_preview_a: gain_uniforms[0],
            gain_preview_b: gain_uniforms[1],
            gain_preview_c: gain_uniforms[2],
            target_size: [body_key.width as f32, body_key.height as f32],
            cursor_ratio: -1.0,
            cursor_width: 1.0,
            cursor_color: [1.0, 1.0, 1.0, 0.92],
        };
        self.ensure_signal_buffer(
            target.device,
            target.queue,
            surface.key,
            SignalBufferCacheKey::new(surface.revision, level_index),
            level.buckets.as_ref(),
            &uniforms,
        );
        let Some(texture_view) = self.ensure_signal_body_texture(
            target.device,
            target.encoder,
            surface.key,
            body_key,
            stats,
        ) else {
            return;
        };
        self.render_texture_view(
            target,
            TextureViewRenderRequest {
                surface,
                texture_identity: GpuSurfaceTextureIdentity::SignalBody(body_key),
                texture_view: &texture_view,
                source: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
                occlusion_regions,
            },
            stats,
        );
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
