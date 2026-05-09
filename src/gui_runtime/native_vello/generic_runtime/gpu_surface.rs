//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::*;
use crate::runtime::{
    GpuSignalSummary, GpuSignalSummaryBucket, GpuSurfaceContent, GpuSurfaceOverlay,
    PaintGpuSurface, PaintPrimitive,
};

mod atlas;
mod encoding;
mod gpu_surface_types;
mod passes;
mod pipeline;
mod resources;
mod signal;
use encoding::*;
pub(super) use gpu_surface_types::GpuSurfaceRenderStats;
use gpu_surface_types::*;
use passes::*;
#[cfg(test)]
pub(super) use pipeline::GPU_SIGNAL_SHADER;

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    textures: HashMap<u64, GpuSurfaceTexture>,
    signal_bodies: HashMap<u64, SignalBodyTexture>,
    signals: HashMap<u64, SignalBuffer>,
    signal_summaries: HashMap<u64, CachedSignalSummary>,
}

impl GpuSurfaceRenderer {
    pub(super) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        target_format: wgpu::TextureFormat,
        target_size: Vector2,
        primitives: &[PaintPrimitive],
    ) -> GpuSurfaceRenderStats {
        let mut stats = GpuSurfaceRenderStats::default();
        for primitive in primitives {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if surface.rect.width() <= 0.0 || surface.rect.height() <= 0.0 {
                continue;
            }
            match &surface.content {
                GpuSurfaceContent::RgbaAtlas { source_rect, atlas } => {
                    if atlas.width == 0 || atlas.height == 0 {
                        continue;
                    }
                    self.render_atlas(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        *source_rect,
                        &mut stats,
                    );
                }
                GpuSurfaceContent::SignalBands { samples, .. } => {
                    if samples.is_empty() {
                        continue;
                    }
                    self.render_signal(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        &mut stats,
                    );
                }
                GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                    if summary.levels.is_empty() {
                        continue;
                    }
                    self.render_signal(
                        device,
                        queue,
                        encoder,
                        target_view,
                        target_format,
                        target_size,
                        surface,
                        &mut stats,
                    );
                }
            }
        }
        stats
    }
}
