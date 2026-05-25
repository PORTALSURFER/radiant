use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::visible_rects_after_occlusion;
use crate::runtime::PaintPrimitive;

mod occlusion;

pub(super) use occlusion::gpu_surface_opaque_suffix_regions;

pub(crate) fn gpu_surface_visible_suffix_regions_into(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
) {
    regions.clear();
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
        let suffix = primitives.get(index + 1..).unwrap_or_default();
        let occlusion_regions = gpu_surface_opaque_suffix_regions(surface.rect, suffix);
        regions.extend(visible_surface_regions(surface.rect, &occlusion_regions));
    }
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
