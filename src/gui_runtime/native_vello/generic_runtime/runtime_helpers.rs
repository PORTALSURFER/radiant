//! Small route, cadence, and profiling helpers for the generic native runner.

use super::GenericRouteOutcome;
use crate::{
    layout::{Rect, Vector2},
    runtime::{GpuHoverCursor, PaintGpuSurface, PaintPrimitive},
};
use std::time::Duration;
use tracing::info;
use winit::event::MouseScrollDelta;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GpuSurfaceInteractionRegion {
    pub(in crate::gui_runtime::native_vello) rect: Rect,
    pub(in crate::gui_runtime::native_vello) fast_pointer_move: bool,
    pub(in crate::gui_runtime::native_vello) coalesce_vertical_wheel: bool,
    pub(in crate::gui_runtime::native_vello) native_hover_cursor: Option<GpuHoverCursor>,
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
            && surface.capabilities.native_hover_cursor.is_none()
        {
            return None;
        }
        Some(Self {
            rect: surface.rect,
            fast_pointer_move: surface.capabilities.fast_pointer_move,
            coalesce_vertical_wheel: surface.capabilities.coalesce_vertical_wheel,
            native_hover_cursor: surface.capabilities.native_hover_cursor,
        })
    }

    pub(in crate::gui_runtime::native_vello) fn contains(
        self,
        point: crate::layout::Point,
    ) -> bool {
        self.rect.contains(point)
    }
}

pub(super) fn collect_gpu_surface_interaction_regions(
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

pub(super) fn animation_frame_interval(target_fps: u32) -> Duration {
    let fps = target_fps.clamp(1, 240);
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

pub(super) fn scroll_delta_to_logical(delta: MouseScrollDelta) -> Vector2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vector2::new(-(x * 40.0), -(y * 40.0)),
        MouseScrollDelta::PixelDelta(position) => {
            Vector2::new(-(position.x as f32), -(position.y as f32))
        }
    }
}

pub(super) fn maybe_log_route_profile(
    reason: &'static str,
    elapsed: Duration,
    outcome: GenericRouteOutcome,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        event_route_us = elapsed.as_micros(),
        routed = outcome.routed,
        redraw_requested = outcome.redraw_requested,
        repaint_requested = outcome.repaint_requested,
        "radiant native input profile"
    );
}

pub(super) fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{ImageRgba, Rgba8};
    use crate::runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface};
    use std::sync::Arc;

    #[test]
    fn gpu_surface_interaction_region_collection_reuses_existing_buffer() {
        let mut regions = Vec::with_capacity(8);
        regions.push(GpuSurfaceInteractionRegion {
            rect: Rect::from_min_size(
                crate::layout::Point::new(99.0, 99.0),
                Vector2::new(1.0, 1.0),
            ),
            fast_pointer_move: true,
            coalesce_vertical_wheel: false,
            native_hover_cursor: None,
        });
        let initial_capacity = regions.capacity();
        let rect = Rect::from_min_size(crate::layout::Point::new(1.0, 2.0), Vector2::new(3.0, 4.0));
        let ignored_rect =
            Rect::from_min_size(crate::layout::Point::new(5.0, 6.0), Vector2::new(7.0, 8.0));
        let native_hover_rect = Rect::from_min_size(
            crate::layout::Point::new(9.0, 10.0),
            Vector2::new(11.0, 12.0),
        );
        let surface = PaintGpuSurface {
            widget_id: 7,
            key: 7,
            revision: 1,
            rect,
            content: crate::runtime::GpuSurfaceContent::RgbaAtlas {
                source_rect: Rect::from_min_size(
                    crate::layout::Point::new(0.0, 0.0),
                    Vector2::new(3.0, 4.0),
                ),
                atlas: Arc::new(ImageRgba::new(3, 4, vec![255; 3 * 4 * 4]).expect("valid image")),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                native_hover_cursor: None,
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
        native_hover_surface.capabilities.native_hover_cursor =
            Some(crate::runtime::GpuHoverCursor {
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
                    native_hover_cursor: None,
                },
                GpuSurfaceInteractionRegion {
                    rect: native_hover_rect,
                    fast_pointer_move: false,
                    coalesce_vertical_wheel: false,
                    native_hover_cursor: Some(crate::runtime::GpuHoverCursor {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 255,
                        },
                        width: 1.0,
                    }),
                }
            ]
        );
        assert_eq!(regions.capacity(), initial_capacity);
    }
}
