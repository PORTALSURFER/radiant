//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::device::{wgpu_device_id, wgpu_target_matches};
use crate::gui::types::{Rect as UiRect, Vector2};
use crate::runtime::{
    GpuSignalGainPreview, GpuSignalSummary, GpuSignalSummaryBucket, GpuSurfaceContent,
    PaintGpuSurface, PaintPrimitive,
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use vello::wgpu;

mod active_keys;
mod atlas;
mod custom_shader;
mod encoding;
mod gpu_surface_types;
mod overlays;
mod passes;
mod pipeline;
mod resources;
mod signal;
mod signal_pipeline;
mod stats;
mod visibility;
use active_keys::ActiveGpuSurfaceKeys;
use encoding::{signal_uniforms_as_bytes, summary_bucket_bytes, summary_bucket_value_count};
use gpu_surface_types::{
    CachedSignalSummary, CustomShaderBinding, CustomShaderPipeline, GpuSurfaceCompositeBinding,
    GpuSurfacePipeline, GpuSurfaceTexture, GpuSurfaceTextureIdentity, SignalBodyCacheKey,
    SignalBodyCacheKeyParts, SignalBodyTexture, SignalBuffer, SignalBufferCacheKey, SignalPipeline,
    SignalUniforms,
};
use passes::{signal_body_render_pass, surface_pixel_extent};
use resources::GpuSurfaceResourceCache;
#[cfg(test)]
pub(super) use signal_pipeline::GPU_SIGNAL_SHADER;
pub(super) use stats::GpuSurfaceRenderStats;
use visibility::gpu_surface_opaque_suffix_regions;
pub(super) use visibility::gpu_surface_visible_suffix_regions_into;

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    pipeline_generation: u64,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    resources: GpuSurfaceResourceCache,
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
        for (index, primitive) in primitives.iter().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if !surface.rect.has_finite_positive_area() {
                continue;
            }
            if !surface.content.is_renderable() {
                continue;
            }
            let occlusion_regions = gpu_surface_opaque_suffix_regions(
                surface.rect,
                primitives.get(index + 1..).unwrap_or_default(),
            );
            match &surface.content {
                GpuSurfaceContent::RgbaAtlas { source_rect, .. } => {
                    self.render_atlas(
                        target,
                        surface,
                        *source_rect,
                        &occlusion_regions,
                        &mut stats,
                    );
                }
                GpuSurfaceContent::SignalBands { .. } => {
                    self.render_signal(target, surface, &occlusion_regions, &mut stats);
                }
                GpuSurfaceContent::SignalSummaryBands { .. } => {
                    self.render_signal(target, surface, &occlusion_regions, &mut stats);
                }
                GpuSurfaceContent::CustomShader { .. } => {
                    self.render_custom_shader(target, surface, &occlusion_regions, &mut stats);
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
        self.resources.prune_inactive(&self.active_keys);
    }

    fn clear_resources(&mut self) {
        self.resources.clear();
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

        assert!(!renderer.resources.signal_summaries.contains_key(&7));
        assert!(renderer.resources.signal_summaries.contains_key(&8));
    }

    #[test]
    fn gpu_surface_renderer_prunes_every_resource_map_to_active_keys() {
        let mut renderer = GpuSurfaceRenderer::default();
        let samples: Arc<[f32]> = [-0.5, 0.25, 0.75, -0.25].into_iter().collect();
        let mut stats = GpuSurfaceRenderStats::default();

        renderer.cached_signal_summary(7, 1, 4, 1, &samples, &mut stats);

        renderer.prune_inactive_resources();

        assert!(renderer.resources.textures.is_empty());
        assert!(renderer.resources.composite_bindings.is_empty());
        assert!(renderer.resources.signal_bodies.is_empty());
        assert!(renderer.resources.signals.is_empty());
        assert!(renderer.resources.signal_summaries.is_empty());
    }
}
