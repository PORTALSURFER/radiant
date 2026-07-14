use crate::{layout::Rect, runtime::PaintPrimitive};

use super::{
    SurfaceOcclusionPlan, SurfaceOcclusionPolicy, SurfaceOcclusionQueryScratch,
    planned_surface_occlusion_regions_into, visible_rects_after_occlusion_into,
};

mod region;

pub(in crate::gui_runtime::native_vello) use region::GpuSurfaceInteractionRegion;

#[cfg(test)]
pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
) {
    let mut plan = SurfaceOcclusionPlan::default();
    plan.preprocess(primitives);
    let mut scratch = GpuSurfaceInteractionScratch::default();
    collect_gpu_surface_interaction_regions_with_scratch(primitives, &plan, regions, &mut scratch);
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct GpuSurfaceInteractionScratch {
    occlusion_regions: Vec<Rect>,
    visible_rects: Vec<Rect>,
    occlusion_scratch: Vec<Rect>,
    query_scratch: SurfaceOcclusionQueryScratch,
}

pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions_with_scratch(
    primitives: &[PaintPrimitive],
    plan: &SurfaceOcclusionPlan,
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
    scratch: &mut GpuSurfaceInteractionScratch,
) {
    regions.clear();
    for (index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        let Some(region) = GpuSurfaceInteractionRegion::from_gpu_surface(index, surface) else {
            continue;
        };
        push_visible_interaction_regions(region, index, plan, regions, scratch);
    }
}

fn push_visible_interaction_regions(
    region: GpuSurfaceInteractionRegion,
    primitive_index: usize,
    plan: &SurfaceOcclusionPlan,
    output: &mut Vec<GpuSurfaceInteractionRegion>,
    scratch: &mut GpuSurfaceInteractionScratch,
) {
    planned_surface_occlusion_regions_into(
        region.rect,
        primitive_index,
        plan,
        SurfaceOcclusionPolicy::GpuCompositor,
        &mut scratch.occlusion_regions,
        &mut scratch.query_scratch,
    );
    if scratch.occlusion_regions.is_empty() {
        output.push(region);
        return;
    }
    visible_rects_after_occlusion_into(
        region.rect,
        scratch.occlusion_regions.iter().copied(),
        &mut scratch.visible_rects,
        &mut scratch.occlusion_scratch,
    );
    output.extend(
        scratch
            .visible_rects
            .iter()
            .copied()
            .map(|rect| GpuSurfaceInteractionRegion { rect, ..region }),
    );
}

#[cfg(test)]
mod tests;
