//! Renderer-frame state owned by the generic native Vello runner.

use super::*;
use crate::{
    runtime::{RetainedSurfaceCachePolicy, SurfacePaintPlan},
    theme::ThemeTokens,
};

pub(super) struct NativeVelloFrameState {
    pub(super) text_renderer: NativeTextRenderer,
    pub(super) scene: Scene,
    pub(super) gpu_surface_renderer: GpuSurfaceRenderer,
    pub(super) post_gpu_overlay_renderer: PostGpuOverlayRenderer,
    pub(super) last_paint_plan: SurfacePaintPlan,
    pub(super) transient_overlay_primitives: Vec<crate::runtime::PaintPrimitive>,
    pub(super) composited_base_frame: Option<CompositedBaseFrame>,
    pub(super) composited_base_dirty: bool,
    pub(super) retained_surface_cache: RetainedSurfaceFrameCache,
    pub(super) last_scene_stats: RetainedSurfaceEncodeStats,
    pub(super) scene_text_runs: SceneTextRunBuffer,
    pub(super) gpu_surface_interaction_regions: Vec<GpuSurfaceInteractionRegion>,
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
            scene_texture_dirty: true,
        }
    }

    pub(super) fn mark_scene_texture_dirty(&mut self) {
        self.scene_texture_dirty = true;
        self.composited_base_dirty = true;
    }

    pub(super) fn mark_composited_base_dirty(&mut self) {
        self.composited_base_dirty = true;
    }
}
