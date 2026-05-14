//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::device::{wgpu_device_id, wgpu_target_matches};
use super::*;
use crate::runtime::{
    GpuSignalSummary, GpuSignalSummaryBucket, GpuSurfaceContent, GpuSurfaceOverlay,
    PaintGpuSurface, PaintPrimitive,
};

mod active_keys;
mod atlas;
mod encoding;
mod gpu_surface_types;
mod passes;
mod pipeline;
mod resources;
mod signal;
mod stats;
use active_keys::ActiveGpuSurfaceKeys;
use encoding::*;
use gpu_surface_types::*;
use passes::*;
#[cfg(test)]
pub(super) use pipeline::GPU_SIGNAL_SHADER;
pub(super) use stats::GpuSurfaceRenderStats;

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    pipeline_generation: u64,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    textures: HashMap<u64, GpuSurfaceTexture>,
    composite_bindings: HashMap<u64, GpuSurfaceCompositeBinding>,
    signal_bodies: HashMap<u64, SignalBodyTexture>,
    signals: HashMap<u64, SignalBuffer>,
    signal_summaries: HashMap<u64, CachedSignalSummary>,
    active_keys: ActiveGpuSurfaceKeys,
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
        self.active_keys.begin_frame();
        for primitive in primitives {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if surface.rect.width() <= 0.0 || surface.rect.height() <= 0.0 {
                continue;
            }
            if !surface.content.is_renderable() {
                continue;
            }
            match &surface.content {
                GpuSurfaceContent::RgbaAtlas { source_rect, .. } => {
                    self.render_atlas(target, surface, *source_rect, &mut stats);
                }
                GpuSurfaceContent::SignalBands { .. } => {
                    self.render_signal(target, surface, &mut stats);
                }
                GpuSurfaceContent::SignalSummaryBands { .. } => {
                    self.render_signal(target, surface, &mut stats);
                }
            }
            self.active_keys.mark_active(surface.key);
        }
        if !self.active_keys.is_empty() {
            self.prune_inactive_resources();
        } else {
            self.clear_resources();
        }
        stats
    }

    fn prune_inactive_resources(&mut self) {
        let active_keys = &self.active_keys;
        self.textures.retain(|key, _| active_keys.contains(key));
        self.composite_bindings
            .retain(|key, _| active_keys.contains(key));
        self.signal_bodies
            .retain(|key, _| active_keys.contains(key));
        self.signals.retain(|key, _| active_keys.contains(key));
        self.signal_summaries
            .retain(|key, _| active_keys.contains(key));
    }

    fn clear_resources(&mut self) {
        self.textures.clear();
        self.composite_bindings.clear();
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

        renderer.active_keys.mark_active(8);
        renderer.prune_inactive_resources();

        assert!(!renderer.signal_summaries.contains_key(&7));
        assert!(renderer.signal_summaries.contains_key(&8));
    }

    #[test]
    fn gpu_surface_renderer_prunes_every_resource_map_to_active_keys() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        renderer.prune_inactive_resources();

        assert!(renderer.textures.is_empty());
        assert!(renderer.composite_bindings.is_empty());
        assert!(renderer.signal_bodies.is_empty());
        assert!(renderer.signals.is_empty());
        assert!(renderer.signal_summaries.is_empty());
    }
}
