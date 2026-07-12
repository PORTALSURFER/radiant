use crate::gui::types::Rect as UiRect;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::intersect_rect;
use crate::runtime::PaintPrimitive;

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn opaque_suffix_regions_into(
    surface_rect: UiRect,
    prefix: &[PaintPrimitive],
    suffix: &[PaintPrimitive],
    regions: &mut Vec<UiRect>,
    clip_stack: &mut Vec<Option<UiRect>>,
) {
    regions.clear();
    clip_stack.clear();
    for primitive in prefix {
        update_clip_stack(primitive, clip_stack);
    }
    for primitive in suffix {
        match primitive {
            PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => {
                update_clip_stack(primitive, clip_stack);
            }
            PaintPrimitive::FillRect(fill) if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA => {
                if let Some(region) = clipped_occlusion_region(surface_rect, fill.rect, clip_stack)
                {
                    regions.push(region);
                }
            }
            PaintPrimitive::FillRectBatch(fill)
                if fill.color.a >= OPAQUE_SUFFIX_OCCLUSION_ALPHA =>
            {
                for rect in fill.rects.iter().copied() {
                    if let Some(region) = clipped_occlusion_region(surface_rect, rect, clip_stack) {
                        regions.push(region);
                    }
                }
            }
            PaintPrimitive::OverlayPanel(panel) => {
                if let Some(region) = clipped_occlusion_region(surface_rect, panel.rect, clip_stack)
                {
                    regions.push(region);
                }
            }
            _ => {}
        }
    }
}

fn update_clip_stack(primitive: &PaintPrimitive, clip_stack: &mut Vec<Option<UiRect>>) {
    match primitive {
        PaintPrimitive::ClipStart(clip) => {
            let clipped = if !clip.rect.has_finite_positive_area() {
                None
            } else {
                match clip_stack.last().copied() {
                    None => Some(clip.rect),
                    Some(Some(parent)) => intersect_rect(parent, clip.rect),
                    Some(None) => None,
                }
            };
            clip_stack.push(clipped);
        }
        PaintPrimitive::ClipEnd(_) => {
            clip_stack.pop();
        }
        _ => {}
    }
}

fn clipped_occlusion_region(
    surface_rect: UiRect,
    fill_rect: UiRect,
    clip_stack: &[Option<UiRect>],
) -> Option<UiRect> {
    if !fill_rect.has_finite_positive_area() {
        return None;
    }
    let fill_rect = match clip_stack.last().copied() {
        None => fill_rect,
        Some(Some(clip)) => intersect_rect(fill_rect, clip)?,
        Some(None) => return None,
    };
    intersect_rect(surface_rect, fill_rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Rgba8, Vector2};

    #[test]
    fn opaque_suffix_regions_ignore_translucent_fills() {
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

        let mut regions = Vec::new();
        let mut clip_stack = Vec::new();
        opaque_suffix_regions_into(surface, &[], &suffix, &mut regions, &mut clip_stack);

        assert_eq!(regions.len(), 1);
        assert_eq!(
            regions[0],
            UiRect::from_min_size(Point::new(30.0, 10.0), Vector2::new(20.0, 20.0))
        );
    }

    #[test]
    fn opaque_suffix_regions_into_reuses_existing_storage() {
        let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
            widget_id: 7,
            rect: UiRect::from_min_size(Point::new(10.0, 10.0), Vector2::new(20.0, 20.0)),
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
        })];
        let mut regions = Vec::with_capacity(8);
        let mut clip_stack = Vec::with_capacity(4);

        opaque_suffix_regions_into(surface, &[], &suffix, &mut regions, &mut clip_stack);
        let capacity = regions.capacity();
        let clip_capacity = clip_stack.capacity();
        opaque_suffix_regions_into(surface, &[], &[], &mut regions, &mut clip_stack);

        assert_eq!(capacity, 8);
        assert_eq!(regions.capacity(), capacity);
        assert_eq!(clip_stack.capacity(), clip_capacity);
        assert!(regions.is_empty());
    }

    #[test]
    fn opaque_suffix_regions_respect_nested_clip_intersections() {
        let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [
            PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: 1,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 80.0)),
            }),
            PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: 2,
                rect: UiRect::from_min_size(Point::new(10.0, 0.0), Vector2::new(90.0, 80.0)),
            }),
            PaintPrimitive::FillRect(crate::runtime::PaintFillRect {
                widget_id: 7,
                rect: surface,
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            }),
            PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 2 }),
            PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
        ];
        let mut regions = Vec::new();
        let mut clip_stack = Vec::new();

        opaque_suffix_regions_into(surface, &[], &suffix, &mut regions, &mut clip_stack);

        assert_eq!(
            regions,
            [UiRect::from_min_size(
                Point::new(10.0, 0.0),
                Vector2::new(30.0, 80.0)
            )]
        );
    }

    #[test]
    fn opaque_suffix_regions_clip_overlay_panels() {
        let surface = UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 80.0));
        let suffix = [
            PaintPrimitive::ClipStart(crate::runtime::PaintClipStart {
                node_id: 1,
                rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 80.0)),
            }),
            PaintPrimitive::OverlayPanel(crate::runtime::PaintOverlayPanel {
                widget_id: 7,
                rect: surface,
                label: None,
                style: crate::widgets::WidgetStyle::default(),
            }),
            PaintPrimitive::ClipEnd(crate::runtime::PaintClipEnd { node_id: 1 }),
        ];
        let mut regions = Vec::new();
        let mut clip_stack = Vec::new();

        opaque_suffix_regions_into(surface, &[], &suffix, &mut regions, &mut clip_stack);

        assert_eq!(
            regions,
            [UiRect::from_min_size(
                Point::new(0.0, 0.0),
                Vector2::new(40.0, 80.0)
            )]
        );
    }
}
