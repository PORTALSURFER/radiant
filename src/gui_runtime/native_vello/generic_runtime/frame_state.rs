//! Renderer-frame state owned by the generic native Vello runner.

use super::{
    CompositedBaseFrame, GpuSurfaceInteractionRegion, GpuSurfaceRenderer, PostGpuOverlayRenderer,
    RetainedSurfaceEncodeStats, RetainedSurfaceFrameCache, SceneTextRunBuffer,
    gpu_surface::gpu_surface_visible_suffix_regions_into, post_gpu_overlay,
};
use crate::{
    gui::types::Rect as UiRect,
    gui_runtime::native_vello::NativeTextRenderer,
    runtime::{PaintPrimitive, RetainedSurfaceCachePolicy, SurfacePaintPlan},
    theme::{DpiScale, ThemeTokens},
};
use vello::{Scene, kurbo::Affine};

pub(super) struct NativeVelloFrameState {
    pub(super) text_renderer: NativeTextRenderer,
    pub(super) scene: Scene,
    scaled_scene: Scene,
    scaled_scene_dpi_scale: DpiScale,
    scaled_scene_dirty: bool,
    pub(super) gpu_surface_renderer: GpuSurfaceRenderer,
    pub(super) post_gpu_overlay_renderer: PostGpuOverlayRenderer,
    pub(super) last_paint_plan: SurfacePaintPlan,
    pub(super) transient_overlay_primitives: Vec<PaintPrimitive>,
    pub(super) composited_base_frame: Option<CompositedBaseFrame>,
    pub(super) composited_base_dirty: bool,
    pub(super) retained_surface_cache: RetainedSurfaceFrameCache,
    pub(super) last_scene_stats: RetainedSurfaceEncodeStats,
    pub(super) scene_text_runs: SceneTextRunBuffer,
    pub(super) gpu_surface_interaction_regions: Vec<GpuSurfaceInteractionRegion>,
    pub(super) post_gpu_overlay_gpu_regions: Vec<UiRect>,
    pub(super) post_gpu_overlay_suffix_start: Option<usize>,
    pub(super) post_gpu_overlay_has_replayable_suffix: bool,
    pub(super) scene_texture_dirty: bool,
}

impl NativeVelloFrameState {
    pub(super) fn new(
        text_renderer: NativeTextRenderer,
        retained_surface_cache: RetainedSurfaceCachePolicy,
    ) -> Self {
        Self {
            text_renderer,
            scene: Scene::new(),
            scaled_scene: Scene::new(),
            scaled_scene_dpi_scale: DpiScale::ONE,
            scaled_scene_dirty: true,
            gpu_surface_renderer: GpuSurfaceRenderer::default(),
            post_gpu_overlay_renderer: PostGpuOverlayRenderer::default(),
            last_paint_plan: SurfacePaintPlan::empty(&ThemeTokens::default()),
            transient_overlay_primitives: Vec::new(),
            composited_base_frame: None,
            composited_base_dirty: true,
            retained_surface_cache: RetainedSurfaceFrameCache::with_policy(retained_surface_cache),
            last_scene_stats: RetainedSurfaceEncodeStats::default(),
            scene_text_runs: SceneTextRunBuffer::new(),
            gpu_surface_interaction_regions: Vec::new(),
            post_gpu_overlay_gpu_regions: Vec::new(),
            post_gpu_overlay_suffix_start: None,
            post_gpu_overlay_has_replayable_suffix: false,
            scene_texture_dirty: true,
        }
    }

    pub(super) fn mark_scene_texture_dirty(&mut self) {
        self.scene_texture_dirty = true;
        self.composited_base_dirty = true;
    }

    pub(super) fn mark_scene_content_dirty(&mut self) {
        self.scaled_scene_dirty = true;
        self.mark_scene_texture_dirty();
    }

    pub(super) fn mark_composited_base_dirty(&mut self) {
        self.composited_base_dirty = true;
    }

    pub(super) fn scene_for_dpi_scale(&mut self, dpi_scale: DpiScale) -> &Scene {
        if dpi_scale == DpiScale::ONE {
            return &self.scene;
        }
        if self.scaled_scene_dirty || self.scaled_scene_dpi_scale != dpi_scale {
            self.scaled_scene.reset();
            self.scaled_scene
                .append(&self.scene, Some(Affine::scale(dpi_scale.factor() as f64)));
            self.scaled_scene_dpi_scale = dpi_scale;
            self.scaled_scene_dirty = false;
        }
        &self.scaled_scene
    }

    pub(super) fn refresh_post_gpu_overlay_cache(&mut self) {
        self.post_gpu_overlay_suffix_start = self
            .last_paint_plan
            .primitives
            .iter()
            .rposition(|primitive| matches!(primitive, PaintPrimitive::GpuSurface(_)))
            .and_then(|index| index.checked_add(1));
        self.post_gpu_overlay_has_replayable_suffix = self
            .post_gpu_overlay_suffix_start
            .and_then(|start| self.last_paint_plan.primitives.get(start..))
            .is_some_and(|suffix| {
                suffix
                    .iter()
                    .any(post_gpu_overlay::geometry::primitive_is_replayable)
            });
        gpu_surface_visible_suffix_regions_into(
            &self.last_paint_plan.primitives,
            &mut self.post_gpu_overlay_gpu_regions,
        );
    }

    pub(super) fn has_post_gpu_overlay_work(&self) -> bool {
        !self.transient_overlay_primitives.is_empty()
            || (self.post_gpu_overlay_has_replayable_suffix
                && !self.post_gpu_overlay_gpu_regions.is_empty())
    }

    pub(super) fn render_post_gpu_overlay(
        &mut self,
        target: &mut post_gpu_overlay::PostGpuOverlayRenderTarget<'_>,
    ) {
        let Self {
            post_gpu_overlay_renderer,
            last_paint_plan,
            transient_overlay_primitives,
            post_gpu_overlay_gpu_regions,
            post_gpu_overlay_suffix_start,
            ..
        } = self;
        let suffix =
            post_gpu_overlay_suffix_start.and_then(|start| last_paint_plan.primitives.get(start..));
        post_gpu_overlay_renderer.render_cached_layers(
            target,
            suffix,
            post_gpu_overlay_gpu_regions,
            transient_overlay_primitives,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_scene_cache_reuses_fractional_dpi_scene_until_content_changes() {
        let mut frame = NativeVelloFrameState::new(
            NativeTextRenderer::new(),
            RetainedSurfaceCachePolicy::default(),
        );
        let dpi_scale = DpiScale::new(1.25);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);

        frame.mark_scene_content_dirty();
        assert!(frame.scaled_scene_dirty);

        let _ = frame.scene_for_dpi_scale(dpi_scale);
        assert!(!frame.scaled_scene_dirty);
    }
}
