use super::*;
use crate::runtime::{GpuSurfaceContent, PaintGpuSurface};
use std::sync::Arc;

impl GpuSurfaceRenderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_signal(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        surface: &PaintGpuSurface,
        stats: &mut GpuSurfaceRenderStats,
    ) {
        let (frames, band_count, frame_range, summary) = match &surface.content {
            GpuSurfaceContent::SignalBands {
                frames,
                band_count,
                frame_range,
                samples,
            } => {
                let summary = self.cached_signal_summary(
                    surface.key,
                    surface.revision,
                    *frames,
                    *band_count,
                    samples,
                );
                (*frames, *band_count, *frame_range, summary)
            }
            GpuSurfaceContent::SignalSummaryBands {
                frames,
                band_count,
                frame_range,
                summary,
            } => (*frames, *band_count, *frame_range, Arc::clone(summary)),
            _ => return,
        };
        let visible = (frame_range[1] - frame_range[0]).max(1.0);
        let frames_per_pixel = visible / surface.rect.width().max(1.0);
        let level_index = summary.level_for_frames_per_pixel(frames_per_pixel);
        let Some(level) = summary.levels.get(level_index) else {
            return;
        };
        let body_key = SignalBodyCacheKey::new(
            surface,
            frames,
            band_count,
            frame_range,
            level.buckets.len(),
            level_index,
        );
        self.ensure_pipeline(device, target_format);
        if self
            .signal_bodies
            .get(&surface.key)
            .is_some_and(|body| body.cache_key == body_key)
        {
            stats.signal_body_cache_hits += 1;
            let Some(body) = self.signal_bodies.get(&surface.key) else {
                return;
            };
            self.render_texture_view(
                device,
                encoder,
                target_view,
                target_format,
                target_size,
                surface,
                &body.view,
                [0.0, 0.0, body_key.width as f32, body_key.height as f32],
                stats,
            );
            return;
        }
        self.ensure_signal_pipeline(device, wgpu::TextureFormat::Rgba8Unorm);
        let uniforms = SignalUniforms {
            dest: [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            frame_range: [
                frame_range[0],
                frame_range[1],
                frames as f32,
                band_count as f32,
            ],
            summary_meta: [
                level.bucket_frames as f32,
                (level.buckets.len() / band_count.max(1)) as f32,
                level_index as f32,
                0.0,
            ],
            target_size: [body_key.width as f32, body_key.height as f32],
            cursor_ratio: -1.0,
            cursor_width: 1.0,
            cursor_color: [1.0, 1.0, 1.0, 0.92],
        };
        self.ensure_signal_buffer(
            device,
            queue,
            surface.key,
            surface.revision ^ ((level_index as u64) << 32),
            level.buckets.as_ref(),
            &uniforms,
        );
        self.ensure_signal_body_texture(device, encoder, surface.key, body_key, stats);
        let Some(body) = self.signal_bodies.get(&surface.key) else {
            return;
        };
        self.render_texture_view(
            device,
            encoder,
            target_view,
            target_format,
            target_size,
            surface,
            &body.view,
            [0.0, 0.0, body_key.width as f32, body_key.height as f32],
            stats,
        );
    }
}
