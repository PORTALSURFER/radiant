use crate::{layout::Rect, runtime::PaintPrimitive};

use super::{intersect_rect, visible_rects_after_occlusion_into};

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
        let suffix = primitives.get(index + 1..).unwrap_or_default();
        push_visible_interaction_regions(
            region,
            suffix,
            regions,
            &mut scratch.opaque_rects,
            &mut scratch.visible_rects,
            &mut scratch.occlusion_scratch,
        );
    }
}

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

fn push_visible_interaction_regions(
    region: GpuSurfaceInteractionRegion,
    suffix: &[PaintPrimitive],
    output: &mut Vec<GpuSurfaceInteractionRegion>,
    opaque_rects: &mut Vec<Rect>,
    visible_rects: &mut Vec<Rect>,
    occlusion_scratch: &mut Vec<Rect>,
) {
    opaque_rects.clear();
    for primitive in suffix {
        push_opaque_fill_rects(region.rect, primitive, opaque_rects);
    }
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

fn push_opaque_fill_rects(surface_rect: Rect, primitive: &PaintPrimitive, output: &mut Vec<Rect>) {
    match primitive {
        PaintPrimitive::FillRect(fill)
            if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA
                && fill.rect.has_finite_positive_area() =>
        {
            if let Some(rect) = intersect_rect(surface_rect, fill.rect) {
                output.push(rect);
            }
        }
        PaintPrimitive::FillRectBatch(fill) if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA => {
            output.extend(
                fill.rects
                    .iter()
                    .copied()
                    .filter(|rect| rect.has_finite_positive_area())
                    .filter_map(|rect| intersect_rect(surface_rect, rect)),
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
