//! Native GPU renderer for retained generic GPU-surface paint primitives.

use super::device::{wgpu_device_id, wgpu_target_matches};
use crate::gui::types::{Rect as UiRect, Vector2};
use crate::runtime::{GpuSurfaceContent, PaintPrimitive};
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
use gpu_surface_types::{GpuSurfacePipeline, SignalPipeline};
use resources::GpuSurfaceResourceCache;
#[cfg(test)]
pub(super) use signal_pipeline::GPU_SIGNAL_SHADER;
pub(super) use stats::GpuSurfaceRenderStats;
use visibility::gpu_surface_opaque_suffix_regions_into;
pub(super) use visibility::{
    GpuSurfaceVisibleSuffixScratch, gpu_surface_visible_suffix_regions_into_with_scratch,
};

#[derive(Default)]
pub(super) struct GpuSurfaceRenderer {
    pipeline: Option<GpuSurfacePipeline>,
    pipeline_generation: u64,
    signal_pipeline: Option<SignalPipeline>,
    signal_pipeline_generation: u64,
    resources: GpuSurfaceResourceCache,
    active_keys: ActiveGpuSurfaceKeys,
    occlusion_regions: Vec<UiRect>,
}

pub(super) struct GpuSurfaceRenderTarget<'a> {
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) target_view: &'a wgpu::TextureView,
    pub(super) format: wgpu::TextureFormat,
    pub(super) size: Vector2,
    pub(super) dpi_scale: crate::theme::DpiScale,
}

impl GpuSurfaceRenderer {
    pub(super) fn render(
        &mut self,
        target: &mut GpuSurfaceRenderTarget<'_>,
        primitives: &[PaintPrimitive],
    ) -> GpuSurfaceRenderStats {
        let mut stats = GpuSurfaceRenderStats::default();
        let mut occlusion_regions = std::mem::take(&mut self.occlusion_regions);
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
            gpu_surface_opaque_suffix_regions_into(
                surface.rect,
                primitives.get(index + 1..).unwrap_or_default(),
                &mut occlusion_regions,
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
        self.occlusion_regions = occlusion_regions;
        stats
    }

    fn prune_inactive_resources(&mut self) {
        self.resources.prune_inactive(&self.active_keys);
    }

    fn clear_resources(&mut self) {
        self.resources.clear();
    }

    #[cfg(test)]
    fn collect_occlusion_regions_for_test(
        &mut self,
        surface_rect: UiRect,
        suffix: &[PaintPrimitive],
    ) -> &[UiRect] {
        gpu_surface_opaque_suffix_regions_into(surface_rect, suffix, &mut self.occlusion_regions);
        &self.occlusion_regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rgba8};
    use std::sync::Arc;

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

    #[test]
    fn gpu_surface_renderer_reuses_occlusion_scratch_storage() {
        let mut renderer = GpuSurfaceRenderer {
            occlusion_regions: Vec::with_capacity(8),
            ..GpuSurfaceRenderer::default()
        };
        let capacity = renderer.occlusion_regions.capacity();
        let surface_rect = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 7,
            rect: UiRect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0)),
            color: Rgba8 {
                r: 47,
                g: 47,
                b: 47,
                a: 255,
            },
        })];

        assert_eq!(
            renderer
                .collect_occlusion_regions_for_test(surface_rect, &suffix)
                .len(),
            1
        );
        assert!(
            renderer
                .collect_occlusion_regions_for_test(surface_rect, &[])
                .is_empty()
        );

        assert_eq!(renderer.occlusion_regions.capacity(), capacity);
    }
}
