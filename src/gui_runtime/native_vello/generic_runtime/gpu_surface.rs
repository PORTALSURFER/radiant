//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::*;
use crate::runtime::{
    GpuSignalSummary, GpuSignalSummaryBucket, GpuSurfaceContent, GpuSurfaceOverlay,
    PaintGpuSurface, PaintPrimitive,
};
use std::collections::HashSet;

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

pub(super) struct GpuSurfaceRenderTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) format: wgpu::TextureFormat,
    pub(super) size: Vector2,
}

impl GpuSurfaceRenderer {
    pub(super) fn render(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        primitives: &[PaintPrimitive],
    ) -> GpuSurfaceRenderStats {
        let mut stats = GpuSurfaceRenderStats::default();
        let mut active_keys = None;
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
                    self.render_atlas(target, surface, *source_rect, &mut stats);
                }
                GpuSurfaceContent::SignalBands { samples, .. } => {
                    if samples.is_empty() {
                        continue;
                    }
                    self.render_signal(target, surface, &mut stats);
                }
                GpuSurfaceContent::SignalSummaryBands { summary, .. } => {
                    if summary.levels.is_empty() {
                        continue;
                    }
                    self.render_signal(target, surface, &mut stats);
                }
            }
            active_keys
                .get_or_insert_with(|| HashSet::with_capacity(primitives.len().min(64)))
                .insert(surface.key);
        }
        if let Some(active_keys) = active_keys {
            self.prune_inactive_resources(&active_keys);
        } else {
            self.clear_resources();
        }
        stats
    }

    fn prune_inactive_resources(&mut self, active_keys: &HashSet<u64>) {
        self.textures.retain(|key, _| active_keys.contains(key));
        self.signal_bodies
            .retain(|key, _| active_keys.contains(key));
        self.signals.retain(|key, _| active_keys.contains(key));
        self.signal_summaries
            .retain(|key, _| active_keys.contains(key));
    }

    fn clear_resources(&mut self) {
        self.textures.clear();
        self.signal_bodies.clear();
        self.signals.clear();
        self.signal_summaries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_surface_renderer_prunes_inactive_signal_summaries() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);
        renderer.cached_signal_summary(8, 1, 4, 1, &samples, &mut stats);

        renderer.prune_inactive_resources(&HashSet::from([8]));

        assert!(!renderer.signal_summaries.contains_key(&7));
        assert!(renderer.signal_summaries.contains_key(&8));
    }

    #[test]
    fn gpu_surface_renderer_prunes_every_resource_map_to_active_keys() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        renderer.prune_inactive_resources(&HashSet::new());

        assert!(renderer.textures.is_empty());
        assert!(renderer.signal_bodies.is_empty());
        assert!(renderer.signals.is_empty());
        assert!(renderer.signal_summaries.is_empty());
    }
}
