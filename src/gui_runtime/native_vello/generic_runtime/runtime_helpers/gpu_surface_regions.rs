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
    let visible =
        visible_rects_after_occlusion(region.rect, suffix.iter().filter_map(opaque_fill_rect));
    output.extend(
        visible
            .into_iter()
            .map(|rect| GpuSurfaceInteractionRegion { rect, ..region }),
    );
}

fn opaque_fill_rect(primitive: &PaintPrimitive) -> Option<Rect> {
    let PaintPrimitive::FillRect(fill) = primitive else {
        return None;
    };
    (fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA && fill.rect.has_finite_positive_area())
        .then_some(fill.rect)
}

#[cfg(test)]
mod tests;
