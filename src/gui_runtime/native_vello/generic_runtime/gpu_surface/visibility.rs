use super::*;
use crate::runtime::PaintPrimitive;

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

pub(super) fn gpu_surface_opaque_suffix_regions(
    surface_rect: UiRect,
    suffix: &[PaintPrimitive],
) -> Vec<UiRect> {
    let mut regions = Vec::new();
    for primitive in suffix {
        let PaintPrimitive::FillRect(fill) = primitive else {
            continue;
        };
        if fill.color.a < OPAQUE_SUFFIX_OCCLUSION_ALPHA {
            continue;
        }
        if let Some(region) = intersect_rect(surface_rect, fill.rect) {
            regions.push(region);
        }
    }
    regions
}

pub(crate) fn gpu_surface_visible_suffix_regions_into(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
) {
    regions.clear();
    for (index, primitive) in primitives.iter().enumerate() {
        let PaintPrimitive::GpuSurface(surface) = primitive else {
            continue;
        };
        if surface.rect.width() <= 0.0 || surface.rect.height() <= 0.0 {
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
    if occlusion_regions.is_empty() {
        return vec![surface_rect];
    }

    let mut visible = vec![surface_rect];
    let mut next = Vec::new();
    for occlusion in occlusion_regions {
        next.clear();
        for rect in visible.drain(..) {
            subtract_rect(rect, *occlusion, &mut next);
        }
        std::mem::swap(&mut visible, &mut next);
        if visible.is_empty() {
            break;
        }
    }
    visible
}

fn subtract_rect(rect: UiRect, occlusion: UiRect, output: &mut Vec<UiRect>) {
    let Some(cut) = intersect_rect(rect, occlusion) else {
        output.push(rect);
        return;
    };

    push_positive_rect(
        output,
        UiRect::from_min_max(rect.min, Point::new(rect.max.x, cut.min.y)),
    );
    push_positive_rect(
        output,
        UiRect::from_min_max(Point::new(rect.min.x, cut.max.y), rect.max),
    );
    push_positive_rect(
        output,
        UiRect::from_min_max(
            Point::new(rect.min.x, cut.min.y),
            Point::new(cut.min.x, cut.max.y),
        ),
    );
    push_positive_rect(
        output,
        UiRect::from_min_max(
            Point::new(cut.max.x, cut.min.y),
            Point::new(rect.max.x, cut.max.y),
        ),
    );
}

fn push_positive_rect(output: &mut Vec<UiRect>, rect: UiRect) {
    if rect.width() > 0.0 && rect.height() > 0.0 {
        output.push(rect);
    }
}

fn intersect_rect(a: UiRect, b: UiRect) -> Option<UiRect> {
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    (max.x > min.x && max.y > min.y).then(|| UiRect::from_min_max(min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn gpu_surface_opaque_suffix_regions_ignore_translucent_fills() {
        let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 7,
                rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 160,
                },
            }),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 8,
                rect: UiRect::from_min_size(Point::new(30.0, 10.0), Vector2::new(20.0, 20.0)),
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            }),
        ];

        let regions = gpu_surface_opaque_suffix_regions(surface, &suffix);

        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0],
            UiRect::from_min_size(Point::new(30.0, 10.0), Vector2::new(20.0, 20.0))
        );
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
