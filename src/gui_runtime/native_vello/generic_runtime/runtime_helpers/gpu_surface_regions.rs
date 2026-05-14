use crate::{
    layout::{Point, Rect},
    runtime::{GpuSurfaceRuntimeOverlays, PaintGpuSurface, PaintPrimitive},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GpuSurfaceInteractionRegion {
    pub(in crate::gui_runtime::native_vello) rect: Rect,
    pub(in crate::gui_runtime::native_vello) fast_pointer_move: bool,
    pub(in crate::gui_runtime::native_vello) coalesce_vertical_wheel: bool,
    pub(in crate::gui_runtime::native_vello) runtime_overlays: GpuSurfaceRuntimeOverlays,
}

impl GpuSurfaceInteractionRegion {
    pub(in crate::gui_runtime::native_vello) fn from_gpu_surface(
        surface: &PaintGpuSurface,
    ) -> Option<Self> {
        if surface.rect.width() <= 0.0
            || surface.rect.height() <= 0.0
            || !surface.content.is_renderable()
        {
            return None;
        }
        if !surface.capabilities.fast_pointer_move
            && !surface.capabilities.coalesce_vertical_wheel
            && surface
                .capabilities
                .runtime_overlays
                .pointer_vertical_line
                .is_none()
        {
            return None;
        }
        Some(Self {
            rect: surface.rect,
            fast_pointer_move: surface.capabilities.fast_pointer_move,
            coalesce_vertical_wheel: surface.capabilities.coalesce_vertical_wheel,
            runtime_overlays: surface.capabilities.runtime_overlays,
        })
    }

    pub(in crate::gui_runtime::native_vello) fn contains(self, point: Point) -> bool {
        self.rect.contains(point)
    }
}

pub(in crate::gui_runtime::native_vello) fn collect_gpu_surface_interaction_regions(
    primitives: &[PaintPrimitive],
    regions: &mut Vec<GpuSurfaceInteractionRegion>,
) {
    regions.clear();
    regions.extend(primitives.iter().filter_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) => {
            GpuSurfaceInteractionRegion::from_gpu_surface(surface)
        }
        _ => None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{ImageRgba, Rgba8, Vector2};
    use crate::runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
    };
    use std::sync::Arc;

    #[test]
    fn gpu_surface_interaction_region_collection_reuses_existing_buffer() {
        let mut regions = Vec::with_capacity(8);
        regions.push(GpuSurfaceInteractionRegion {
            rect: Rect::from_min_size(Point::new(99.0, 99.0), Vector2::new(1.0, 1.0)),
            fast_pointer_move: true,
            coalesce_vertical_wheel: false,
            runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
        });
        let initial_capacity = regions.capacity();
        let rect = Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(3.0, 4.0));
        let ignored_rect = Rect::from_min_size(Point::new(5.0, 6.0), Vector2::new(7.0, 8.0));
        let native_hover_rect =
            Rect::from_min_size(Point::new(9.0, 10.0), Vector2::new(11.0, 12.0));
        let surface = PaintGpuSurface {
            widget_id: 7,
            key: 7,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(3.0, 4.0)),
                atlas: Arc::new(ImageRgba::new(3, 4, vec![255; 3 * 4 * 4]).expect("valid image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            overlays: Vec::new(),
        };
        let mut ignored_surface = surface.clone();
        ignored_surface.rect = ignored_rect;
        ignored_surface.capabilities.fast_pointer_move = false;
        ignored_surface.capabilities.coalesce_vertical_wheel = false;
        let mut invalid_surface = surface.clone();
        invalid_surface.content = GpuSurfaceContent::SignalBands {
            frames: 1,
            band_count: 0,
            frame_range: [0.0, 1.0],
            samples: Arc::<[f32]>::from([0.0]),
        };
        let mut native_hover_surface = surface.clone();
        native_hover_surface.rect = native_hover_rect;
        native_hover_surface.capabilities.fast_pointer_move = false;
        native_hover_surface.capabilities.coalesce_vertical_wheel = false;
        native_hover_surface.capabilities.runtime_overlays =
            GpuSurfaceRuntimeOverlays::pointer_vertical_line(GpuSurfaceLineStyle {
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            });
        let primitives = [
            PaintPrimitive::GpuSurface(ignored_surface),
            PaintPrimitive::GpuSurface(invalid_surface),
            PaintPrimitive::GpuSurface(surface),
            PaintPrimitive::GpuSurface(native_hover_surface),
        ];

        collect_gpu_surface_interaction_regions(&primitives, &mut regions);

        assert_eq!(
            regions,
            [
                GpuSurfaceInteractionRegion {
                    rect,
                    fast_pointer_move: true,
                    coalesce_vertical_wheel: true,
                    runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
                },
                GpuSurfaceInteractionRegion {
                    rect: native_hover_rect,
                    fast_pointer_move: false,
                    coalesce_vertical_wheel: false,
                    runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                        GpuSurfaceLineStyle {
                            color: Rgba8 {
                                r: 255,
                                g: 255,
                                b: 255,
                                a: 255,
                            },
                            width: 1.0,
                        },
                    ),
                }
            ]
        );
        assert_eq!(regions.capacity(), initial_capacity);
    }
}
