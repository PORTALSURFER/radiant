use crate::{layout::Rect, runtime::PaintPrimitive};

use super::{
    SurfaceOcclusionPolicy, surface_occlusion_regions_into, visible_rects_after_occlusion_into,
};

mod region;

pub(in crate::gui_runtime::native_vello) use region::GpuSurfaceInteractionRegion;

#[cfg(test)]
pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
) {
    let mut scratch = GpuSurfaceInteractionScratch::default();
    collect_gpu_surface_interaction_regions_with_scratch(primitives, regions, &mut scratch);
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct GpuSurfaceInteractionScratch {
    opaque_rects: Vec<Rect>,
    visible_rects: Vec<Rect>,
    occlusion_scratch: Vec<Rect>,
    clip_stack: Vec<Option<Rect>>,
}

pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions_with_scratch(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
    scratch: &mut GpuSurfaceInteractionScratch,
) {
    regions.clear();
    for (index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        let Some(region) = GpuSurfaceInteractionRegion::from_gpu_surface(surface) else {
            continue;
        };
        push_visible_interaction_regions(
            region,
            primitives.get(..index).unwrap_or_default(),
            primitives.get(index + 1..).unwrap_or_default(),
            regions,
            &mut scratch.opaque_rects,
            &mut scratch.visible_rects,
            &mut scratch.occlusion_scratch,
            &mut scratch.clip_stack,
        );
    }
}

fn push_visible_interaction_regions(
    region: GpuSurfaceInteractionRegion,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    output: &mut Vec<GpuSurfaceInteractionRegion>,
    opaque_rects: &mut Vec<Rect>,
    visible_rects: &mut Vec<Rect>,
    occlusion_scratch: &mut Vec<Rect>,
    clip_stack: &mut Vec<Option<Rect>>,
) {
    surface_occlusion_regions_into(
        region.rect,
        prefix,
        suffix,
        SurfaceOcclusionPolicy::GpuCompositor,
        opaque_rects,
        clip_stack,
    );
    if opaque_rects.is_empty() {
        output.push(region);
        return;
    }
    visible_rects_after_occlusion_into(
        region.rect,
        opaque_rects.iter().copied(),
        visible_rects,
        occlusion_scratch,
    );
    output.extend(
        visible_rects
            .iter()
            .copied()
            .map(|rect| GpuSurfaceInteractionRegion { rect, ..region }),
    );
}

#[cfg(test)]
mod tests;
