use super::super::*;
use crate::gui_runtime::native_vello::generic_runtime::runtime_helpers::intersect_rect;
use crate::runtime::PaintPrimitive;

const OPAQUE_SUFFIX_OCCLUSION_ALPHA: u8 = 240;

pub(in crate::gui_runtime::native_vello::generic_runtime::gpu_surface) fn gpu_surface_opaque_suffix_regions(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::Point;

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
}
