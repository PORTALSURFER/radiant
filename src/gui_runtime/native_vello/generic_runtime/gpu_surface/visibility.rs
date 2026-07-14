use super::custom_shader::custom_shader_descriptor_is_supported;
use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::{
    SurfaceOcclusionPlan, SurfaceOcclusionPolicy, SurfaceOcclusionQueryScratch,
    SurfaceOcclusionQueryStats, intersect_rect, planned_surface_occlusion_regions_into,
    visible_rects_after_occlusion, visible_rects_after_occlusion_into,
};
use crate::runtime::{PaintGpuSurface, PaintPrimitive};

#[cfg(test)]
pub(crate) fn gpu_surface_visible_suffix_regions_into(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
) {
    let mut plan = SurfaceOcclusionPlan::default();
    plan.preprocess(primitives);
    let mut scratch = SurfaceVisibleSuffixScratch::default();
    gpu_surface_visible_suffix_regions_into_with_scratch(primitives, &plan, regions, &mut scratch);
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct SurfaceVisibleSuffixScratch {
    occlusion_regions: Vec<UiRect>,
    visible_regions: Vec<UiRect>,
    occlusion_scratch: Vec<UiRect>,
    query_scratch: SurfaceOcclusionQueryScratch,
    query_stats: SurfaceOcclusionQueryStats,
    gpu_surfaces_planned: usize,
    visible_gpu_surfaces: usize,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn gpu_surface_visible_suffix_regions_into_with_scratch(
    primitives: &[PaintPrimitive],
    plan: &SurfaceOcclusionPlan,
    regions: &mut Vec<UiRect>,
    scratch: &mut SurfaceVisibleSuffixScratch,
) {
    regions.clear();
    scratch.query_stats = SurfaceOcclusionQueryStats::default();
    scratch.gpu_surfaces_planned = 0;
    scratch.visible_gpu_surfaces = 0;
    for (index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        scratch.gpu_surfaces_planned = scratch.gpu_surfaces_planned.saturating_add(1);
        if gpu_surface_requires_compositing(surface, index, plan, scratch) {
            scratch.visible_gpu_surfaces = scratch.visible_gpu_surfaces.saturating_add(1);
            regions.extend(scratch.visible_regions.iter().copied());
        }
    }
}

pub(in crate::gui_runtime::native_vello) fn gpu_surface_requires_compositing(
    surface: &PaintGpuSurface,
    primitive_index: usize,
    plan: &SurfaceOcclusionPlan,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    gpu_surface_requires_compositing_in_viewport(
        surface,
        primitive_index,
        surface.rect,
        plan,
        scratch,
    )
}

pub(in crate::gui_runtime::native_vello) fn gpu_surface_requires_compositing_in_viewport(
    surface: &PaintGpuSurface,
    primitive_index: usize,
    viewport: UiRect,
    plan: &SurfaceOcclusionPlan,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    scratch.visible_regions.clear();
    if !surface.rect.has_finite_positive_area() || !surface.content.is_retained_renderable() {
        return false;
    }
    if let crate::runtime::GpuSurfaceContent::CustomShader { descriptor } = &surface.content
        && !custom_shader_descriptor_is_supported(descriptor)
    {
        return false;
    }
    surface_rect_has_visible_region_in_viewport(
        surface.rect,
        primitive_index,
        viewport,
        plan,
        SurfaceOcclusionPolicy::GpuCompositor,
        scratch,
    )
}

pub(in crate::gui_runtime::native_vello) fn surface_rect_has_visible_region_in_viewport(
    surface_rect: UiRect,
    primitive_index: usize,
    viewport: UiRect,
    plan: &SurfaceOcclusionPlan,
    policy: SurfaceOcclusionPolicy,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    scratch.visible_regions.clear();
    let Some(surface_rect) = intersect_rect(surface_rect, viewport) else {
        return false;
    };
    surface_rect_has_visible_region(surface_rect, primitive_index, plan, policy, scratch)
}

pub(in crate::gui_runtime::native_vello) fn surface_rect_has_visible_region(
    surface_rect: UiRect,
    primitive_index: usize,
    plan: &SurfaceOcclusionPlan,
    policy: SurfaceOcclusionPolicy,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    scratch.visible_regions.clear();
    if !surface_rect.has_finite_positive_area() {
        return false;
    }
    let query_stats = planned_surface_occlusion_regions_into(
        surface_rect,
        primitive_index,
        plan,
        policy,
        &mut scratch.occlusion_regions,
        &mut scratch.query_scratch,
    );
    scratch.query_stats.add(query_stats);
    if scratch.occlusion_regions.is_empty() {
        scratch.visible_regions.push(surface_rect);
        return true;
    }
    visible_rects_after_occlusion_into(
        surface_rect,
        scratch.occlusion_regions.iter().copied(),
        &mut scratch.visible_regions,
        &mut scratch.occlusion_scratch,
    );
    !scratch.visible_regions.is_empty()
}

pub(in crate::gui_runtime::native_vello) fn surface_occlusion_query_stats(
    scratch: &SurfaceVisibleSuffixScratch,
) -> SurfaceOcclusionQueryStats {
    scratch.query_stats
}

/// Reusable allocation storage for compositor occlusion performance scenarios.
#[doc(hidden)]
#[derive(Default)]
pub struct GpuSurfaceOcclusionPlanningScratch {
    plan: SurfaceOcclusionPlan,
    visibility: SurfaceVisibleSuffixScratch,
    visible_regions: Vec<UiRect>,
}

/// Execute the native compositor's clip, suffix-occlusion, and visible-region planner.
#[doc(hidden)]
pub fn plan_gpu_surface_occlusion_for_diagnostics(
    primitives: &[PaintPrimitive],
    scratch: &mut GpuSurfaceOcclusionPlanningScratch,
) -> crate::runtime::GpuSurfaceOcclusionPlanningDiagnostics {
    scratch.plan.preprocess(primitives);
    gpu_surface_visible_suffix_regions_into_with_scratch(
        primitives,
        &scratch.plan,
        &mut scratch.visible_regions,
        &mut scratch.visibility,
    );
    let plan_stats = scratch.plan.stats();
    let query_stats = surface_occlusion_query_stats(&scratch.visibility);
    crate::runtime::GpuSurfaceOcclusionPlanningDiagnostics {
        paint_primitives_visited: plan_stats.paint_primitives_visited,
        occluder_rects_indexed: plan_stats.occluder_rects_indexed,
        gpu_surfaces_planned: scratch.visibility.gpu_surfaces_planned,
        visible_gpu_surfaces: scratch.visibility.visible_gpu_surfaces,
        index_nodes_visited: query_stats.index_nodes_visited,
        occluder_candidates_visited: query_stats.occluder_candidates_visited,
        visible_regions_produced: scratch.visible_regions.len(),
    }
}

pub(crate) fn visible_surface_regions(
    surface_rect: UiRect,
    occlusion_regions: &[UiRect],
) -> Vec<UiRect> {
    visible_rects_after_occlusion(surface_rect, occlusion_regions.iter().copied())
}

#[cfg(test)]
#[path = "visibility/tests.rs"]
mod tests;
