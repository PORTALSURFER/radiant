use super::custom_shader::custom_shader_descriptor_is_supported;
use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::{
    SurfaceOcclusionPolicy, intersect_rect, surface_occlusion_regions_into,
    visible_rects_after_occlusion, visible_rects_after_occlusion_into,
};
use crate::runtime::{PaintGpuSurface, PaintPrimitive};

#[cfg(test)]
pub(crate) fn gpu_surface_visible_suffix_regions_into(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
) {
    let mut scratch = SurfaceVisibleSuffixScratch::default();
    gpu_surface_visible_suffix_regions_into_with_scratch(primitives, regions, &mut scratch);
}

#[derive(Default)]
pub(in crate::gui_runtime::native_vello) struct SurfaceVisibleSuffixScratch {
    occlusion_regions: Vec<UiRect>,
    visible_regions: Vec<UiRect>,
    occlusion_scratch: Vec<UiRect>,
    clip_stack: Vec<Option<UiRect>>,
}

pub(in crate::gui_runtime::native_vello::generic_runtime) fn gpu_surface_visible_suffix_regions_into_with_scratch(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
    scratch: &mut SurfaceVisibleSuffixScratch,
) {
    regions.clear();
    for (index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        let suffix = primitives.get(index + 1..).unwrap_or_default();
        let prefix = primitives.get(..index).unwrap_or_default();
        if gpu_surface_requires_compositing(surface, prefix, suffix, scratch) {
            regions.extend(scratch.visible_regions.iter().copied());
        }
    }
}

pub(in crate::gui_runtime::native_vello) fn gpu_surface_requires_compositing(
    surface: &PaintGpuSurface,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    gpu_surface_requires_compositing_in_viewport(surface, surface.rect, prefix, suffix, scratch)
}

pub(in crate::gui_runtime::native_vello) fn gpu_surface_requires_compositing_in_viewport(
    surface: &PaintGpuSurface,
    viewport: UiRect,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
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
        viewport,
        prefix,
        suffix,
        SurfaceOcclusionPolicy::GpuCompositor,
        scratch,
    )
}

pub(in crate::gui_runtime::native_vello) fn surface_rect_has_visible_region_in_viewport(
    surface_rect: UiRect,
    viewport: UiRect,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    policy: SurfaceOcclusionPolicy,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    scratch.visible_regions.clear();
    let Some(surface_rect) = intersect_rect(surface_rect, viewport) else {
        return false;
    };
    surface_rect_has_visible_region(surface_rect, prefix, suffix, policy, scratch)
}

pub(in crate::gui_runtime::native_vello) fn surface_rect_has_visible_region(
    surface_rect: UiRect,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    policy: SurfaceOcclusionPolicy,
    scratch: &mut SurfaceVisibleSuffixScratch,
) -> bool {
    scratch.visible_regions.clear();
    if !surface_rect.has_finite_positive_area() {
        return false;
    }
    surface_occlusion_regions_into(
        surface_rect,
        prefix,
        suffix,
        policy,
        &mut scratch.occlusion_regions,
        &mut scratch.clip_stack,
    );
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

pub(crate) fn visible_surface_regions(
    surface_rect: UiRect,
    occlusion_regions: &[UiRect],
) -> Vec<UiRect> {
    visible_rects_after_occlusion(surface_rect, occlusion_regions.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rgba8, Vector2};
    use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::intersect_rect;
    use crate::runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface};
    use std::sync::Arc;

    #[test]
    fn visible_surface_regions_remove_opaque_overlay_rectangles() {
        let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let occlusion = UiRect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0));

        let regions = visible_surface_regions(surface, &[occlusion]);

        assert_eq!(regions.len(), 4);
        assert!(regions.iter().all(|region| region.width() > 0.0));
        assert!(regions.iter().all(|region| region.height() > 0.0));
        assert!(!regions.contains(&occlusion));
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_reuse_existing_storage() {
        let primitives = [gpu_surface(1), translucent_fill(2), gpu_surface(3)];
        let mut regions = Vec::with_capacity(8);

        gpu_surface_visible_suffix_regions_into(&primitives, &mut regions);
        let capacity = regions.capacity();
        gpu_surface_visible_suffix_regions_into(&[gpu_surface(4)], &mut regions);

        assert_eq!(capacity, 8);
        assert_eq!(regions.capacity(), capacity);
        assert_eq!(regions.len(), 1);
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_reuse_scratch_storage() {
        let primitives = [
            gpu_surface(1),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 2,
                rect: UiRect::from_min_size(Point::new(20.0, 15.0), Vector2::new(50.0, 30.0)),
                color: Rgba8 {
                    r: 47,
                    g: 47,
                    b: 47,
                    a: 255,
                },
            }),
        ];
        let mut regions = Vec::new();
        let mut scratch = SurfaceVisibleSuffixScratch {
            occlusion_regions: Vec::with_capacity(8),
            visible_regions: Vec::with_capacity(8),
            occlusion_scratch: Vec::with_capacity(8),
            clip_stack: Vec::with_capacity(8),
        };
        let occlusion_capacity = scratch.occlusion_regions.capacity();
        let visible_capacity = scratch.visible_regions.capacity();
        let occlusion_scratch_capacity = scratch.occlusion_scratch.capacity();
        let clip_stack_capacity = scratch.clip_stack.capacity();

        gpu_surface_visible_suffix_regions_into_with_scratch(
            &primitives,
            &mut regions,
            &mut scratch,
        );
        gpu_surface_visible_suffix_regions_into_with_scratch(
            &[gpu_surface(3)],
            &mut regions,
            &mut scratch,
        );

        assert_eq!(scratch.occlusion_regions.capacity(), occlusion_capacity);
        assert_eq!(scratch.visible_regions.capacity(), visible_capacity);
        assert_eq!(
            scratch.occlusion_scratch.capacity(),
            occlusion_scratch_capacity
        );
        assert_eq!(scratch.clip_stack.capacity(), clip_stack_capacity);
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_keep_full_rect_without_occluders() {
        let primitives = [gpu_surface(1), translucent_fill(2)];
        let mut regions = Vec::new();

        gpu_surface_visible_suffix_regions_into(&primitives, &mut regions);

        assert_eq!(
            regions,
            [UiRect::from_min_size(
                Point::new(0.0, 0.0),
                Vector2::new(100.0, 80.0)
            )]
        );
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_skip_nonfinite_surface_rects() {
        let mut invalid = gpu_surface(1);
        let PaintPrimitive::GpuSurface(surface) = &mut invalid else {
            panic!("expected gpu surface");
        };
        surface.rect =
            UiRect::from_min_max(Point::new(f32::NEG_INFINITY, 0.0), Point::new(1.0, 1.0));
        let mut regions = Vec::new();

        gpu_surface_visible_suffix_regions_into(&[invalid, gpu_surface(2)], &mut regions);

        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0],
            UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0))
        );
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_intersect_active_prefix_clips() {
        let clip = UiRect::from_min_size(Point::new(20.0, 10.0), Vector2::new(50.0, 30.0));
        let primitives = [
            PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: 1,
                rect: clip,
            }),
            gpu_surface(2),
            PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
        ];
        let mut regions = Vec::new();

        gpu_surface_visible_suffix_regions_into(&primitives, &mut regions);

        assert_eq!(regions, [clip]);
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_skip_covered_clipped_portions() {
        let clip = UiRect::from_min_size(Point::new(20.0, 10.0), Vector2::new(50.0, 30.0));
        let primitives = [
            PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: 1,
                rect: clip,
            }),
            gpu_surface(2),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 3,
                rect: clip,
                color: Rgba8 {
                    r: 47,
                    g: 47,
                    b: 47,
                    a: 255,
                },
            }),
            PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
        ];
        let mut regions = Vec::new();

        gpu_surface_visible_suffix_regions_into(&primitives, &mut regions);

        assert!(regions.is_empty());
    }

    #[test]
    fn gpu_surface_visible_suffix_regions_remove_later_opaque_panels() {
        let primitives = [
            gpu_surface(1),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 2,
                rect: UiRect::from_min_size(Point::new(45.0, 0.0), Vector2::new(2.0, 80.0)),
                color: Rgba8 {
                    r: 255,
                    g: 142,
                    b: 92,
                    a: 230,
                },
            }),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 3,
                rect: UiRect::from_min_size(Point::new(30.0, 20.0), Vector2::new(40.0, 30.0)),
                color: Rgba8 {
                    r: 47,
                    g: 47,
                    b: 47,
                    a: 255,
                },
            }),
        ];
        let mut regions = Vec::new();

        gpu_surface_visible_suffix_regions_into(&primitives, &mut regions);

        assert_eq!(regions.len(), 4);
        let panel = UiRect::from_min_size(Point::new(30.0, 20.0), Vector2::new(40.0, 30.0));
        assert!(
            regions
                .iter()
                .all(|region| intersect_rect(*region, panel).is_none())
        );
    }

    fn gpu_surface(widget_id: u64) -> PaintPrimitive {
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id,
            key: widget_id,
            revision: 0,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0)),
            content: GpuSurfaceContent::RgbaAtlas {
                atlas: Arc::new(
                    crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                        .expect("valid one-pixel image"),
                ),
                source_rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })
    }

    fn translucent_fill(widget_id: u64) -> PaintPrimitive {
        PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id,
            rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(10.0, 10.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 160,
            },
        })
    }
}
