use crate::{layout::Rect, runtime::PaintPrimitive};

use super::visible_rects_after_occlusion;

mod region;

pub(in crate::gui_runtime::native_vello) use region::GpuSurfaceInteractionRegion;

pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
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
        push_visible_interaction_regions(region, suffix, regions);
    }
}

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

fn push_visible_interaction_regions(
    region: GpuSurfaceInteractionRegion,
    suffix: &[PaintPrimitive],
    output: &mut Vec<GpuSurfaceInteractionRegion>,
) {
    let mut opaque_rects = Vec::new();
    for primitive in suffix {
        push_opaque_fill_rects(primitive, &mut opaque_rects);
    }
    let visible = visible_rects_after_occlusion(region.rect, opaque_rects.iter().copied());
    output.extend(
        visible
            .into_iter()
            .map(|rect| GpuSurfaceInteractionRegion { rect, ..region }),
    );
}

fn push_opaque_fill_rects(primitive: &PaintPrimitive, output: &mut Vec<Rect>) {
    match primitive {
        PaintPrimitive::FillRect(fill)
            if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA
                && fill.rect.has_finite_positive_area() =>
        {
            output.push(fill.rect);
        }
        PaintPrimitive::FillRectBatch(fill) if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA => {
            output.extend(
                fill.rects
                    .iter()
                    .copied()
                    .filter(|rect| rect.has_finite_positive_area()),
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;
