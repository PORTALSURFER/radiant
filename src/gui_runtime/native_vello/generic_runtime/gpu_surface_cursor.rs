use crate::{
    gui::types::Point,
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive},
};

pub(super) fn topmost_native_hover_surface_index(
    primitives: &[PaintPrimitive],
    position: Point,
) -> Option<usize> {
    primitives.iter().rposition(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) => {
            surface
                .capabilities
                .runtime_overlays
                .pointer_vertical_line
                .is_some()
                && surface.rect.width() > 0.0
                && surface.rect.height() > 0.0
                && surface.content.is_renderable()
                && surface.rect.contains(position)
        }
        _ => false,
    })
}

pub(super) fn update_surface_cursor_overlay(
    surface: &mut PaintGpuSurface,
    position: Point,
) -> bool {
    let Some(cursor) = surface.capabilities.runtime_overlays.pointer_vertical_line else {
        return false;
    };
    let ratio = ((position.x - surface.rect.min.x) / surface.rect.width().max(1.0)).clamp(0.0, 1.0);
    let mut cursor_count = 0;
    let mut cursor_is_current = false;
    for overlay in &surface.overlays {
        let GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio: current_ratio,
            color,
            width,
        } = overlay
        else {
            continue;
        };
        cursor_count += 1;
        cursor_is_current |=
            *current_ratio == ratio && *color == cursor.color && *width == cursor.width;
    }
    if cursor_count == 1 && cursor_is_current {
        return false;
    }
    clear_surface_cursor_overlay(surface);
    surface
        .overlays
        .push(GpuSurfaceOverlay::RuntimeVerticalLine {
            ratio,
            color: cursor.color,
            width: cursor.width,
        });
    true
}

pub(super) fn clear_surface_cursor_overlay(surface: &mut PaintGpuSurface) -> bool {
    let previous_len = surface.overlays.len();
    surface
        .overlays
        .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::RuntimeVerticalLine { .. }));
    previous_len != surface.overlays.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{ImageRgba, Rect, Rgba8, Vector2},
        runtime::{
            GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle,
            GpuSurfaceRuntimeOverlays,
        },
    };
    use std::sync::Arc;

    #[test]
    fn gpu_surface_lookup_skips_unrenderable_surface_content() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
        let capabilities = GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
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
        };
        let primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 1,
                revision: 1,
                rect,
                content: GpuSurfaceContent::SignalBands {
                    frames: 1,
                    band_count: 0,
                    frame_range: [0.0, 1.0],
                    samples: Arc::<[f32]>::from([0.0]),
                },
                capabilities,
                overlays: Vec::new(),
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 2,
                revision: 1,
                rect,
                content: GpuSurfaceContent::RgbaAtlas {
                    source_rect: Rect::from_min_size(
                        Point::new(0.0, 0.0),
                        Vector2::new(20.0, 20.0),
                    ),
                    atlas: Arc::new(
                        ImageRgba::new(20, 20, vec![255; 20 * 20 * 4]).expect("valid image"),
                    ),
                },
                capabilities,
                overlays: Vec::new(),
            }),
        ];

        let surface_index = topmost_native_hover_surface_index(&primitives, Point::new(10.0, 10.0))
            .expect("valid surface");

        assert_eq!(surface_index, 1);
    }

    #[test]
    fn gpu_surface_lookup_skips_empty_surface_rects() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 20.0));
        let primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: 1,
            key: 1,
            revision: 1,
            rect,
            content: GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::default(),
            },
            overlays: Vec::new(),
        })];

        assert!(topmost_native_hover_surface_index(&primitives, Point::new(0.0, 10.0)).is_none());
    }

    #[test]
    fn native_hover_cursor_updates_topmost_surface_and_clears_stale_cursors() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
        let capabilities = GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: false,
            runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                GpuSurfaceLineStyle {
                    color: Rgba8 {
                        r: 255,
                        g: 160,
                        b: 0,
                        a: 255,
                    },
                    width: 2.0,
                },
            ),
        };
        let content = GpuSurfaceContent::RgbaAtlas {
            source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
            atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
        };
        let mut primitives = vec![
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 1,
                key: 1,
                revision: 1,
                rect,
                content: content.clone(),
                capabilities,
                overlays: vec![GpuSurfaceOverlay::RuntimeVerticalLine {
                    ratio: 0.1,
                    color: capabilities
                        .runtime_overlays
                        .pointer_vertical_line
                        .unwrap()
                        .color,
                    width: 2.0,
                }],
            }),
            PaintPrimitive::GpuSurface(PaintGpuSurface {
                widget_id: 2,
                key: 2,
                revision: 1,
                rect,
                content,
                capabilities,
                overlays: Vec::new(),
            }),
        ];

        let target = topmost_native_hover_surface_index(&primitives, Point::new(30.0, 10.0));

        assert_eq!(target, Some(1));
        for (index, primitive) in primitives.iter_mut().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if Some(index) == target {
                assert!(update_surface_cursor_overlay(
                    surface,
                    Point::new(30.0, 10.0)
                ));
            } else {
                assert!(clear_surface_cursor_overlay(surface));
            }
        }
        let [
            PaintPrimitive::GpuSurface(bottom),
            PaintPrimitive::GpuSurface(top),
        ] = primitives.as_slice()
        else {
            panic!("expected GPU surfaces");
        };
        assert!(bottom.overlays.is_empty());
        assert!(matches!(
            top.overlays.as_slice(),
            [GpuSurfaceOverlay::RuntimeVerticalLine { ratio, .. }] if *ratio == 0.75
        ));
    }
}
