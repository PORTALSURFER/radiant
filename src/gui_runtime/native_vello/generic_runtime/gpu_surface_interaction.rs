//! Interaction fast paths for retained GPU surface primitives.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, RenderFrameProfile, maybe_log_route_profile,
};
use crate::{
    gui::types::{Point, Vector2},
    runtime::{GpuSurfaceOverlay, PaintGpuSurface, PaintPrimitive},
};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingGpuSurfaceWheel {
    pub(super) position: Point,
    pub(super) delta: Vector2,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn handle_gpu_surface_route_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        position: Point,
        delta: Vector2,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_route(position, delta) {
            self.deferred_surface_refresh = true;
            self.request_redraw_if_needed();
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    pub(super) fn queue_gpu_surface_wheel(&mut self, position: Point, delta: Vector2) {
        match &mut self.pending_gpu_surface_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
            }
            None => {
                self.pending_gpu_surface_wheel = Some(PendingGpuSurfaceWheel { position, delta });
            }
        }
        self.update_gpu_surface_cursor_overlay(position);
        self.request_redraw_if_needed();
    }

    pub(super) fn flush_pending_gpu_surface_wheel(&mut self, profile: &mut RenderFrameProfile) {
        let Some(pending) = self.pending_gpu_surface_wheel.take() else {
            return;
        };
        let started = Instant::now();
        let outcome = self
            .core
            .route_scroll_deferred_refresh(pending.position, pending.delta);
        profile.coalesced_wheel_route = started.elapsed();
        maybe_log_route_profile("coalesced_wheel", profile.coalesced_wheel_route, outcome);
        if outcome.needs_redraw() {
            self.deferred_surface_refresh = true;
        }
    }

    pub(super) fn handle_gpu_surface_pointer_move_outcome(
        &mut self,
        outcome: GenericRouteOutcome,
        previous: Option<Point>,
        position: Point,
    ) {
        if !outcome.needs_redraw() {
            return;
        }
        if self.can_fast_path_gpu_surface_pointer_move(previous, position) {
            if self.update_gpu_surface_cursor_overlay(position) {
                self.request_redraw_if_needed();
            }
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    pub(super) fn can_fast_path_gpu_surface_route(&self, position: Point, delta: Vector2) -> bool {
        let is_horizontal_pan = delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON;
        !is_horizontal_pan && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn can_fast_path_gpu_surface_pointer_move(
        &self,
        previous: Option<Point>,
        position: Point,
    ) -> bool {
        let Some(previous) = previous else {
            return false;
        };
        self.gpu_surface_interaction_regions.iter().any(|region| {
            region.fast_pointer_move && region.contains(previous) && region.contains(position)
        })
    }

    pub(super) fn paint_plan_has_coalescing_gpu_surface_at(&self, position: Point) -> bool {
        self.gpu_surface_interaction_regions
            .iter()
            .any(|region| region.coalesce_vertical_wheel && region.contains(position))
    }

    pub(super) fn native_hover_surface_contains(&self, position: Point) -> bool {
        self.gpu_surface_interaction_regions
            .iter()
            .any(|region| region.native_hover_cursor.is_some() && region.contains(position))
    }

    pub(super) fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn update_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let Some(surface) = gpu_surface_at_mut(&mut self.last_paint_plan.primitives, position)
        else {
            return false;
        };
        let Some(cursor) = surface.capabilities.native_hover_cursor else {
            return false;
        };
        let ratio =
            ((position.x - surface.rect.min.x) / surface.rect.width().max(1.0)).clamp(0.0, 1.0);
        let mut cursor_count = 0;
        let mut cursor_is_current = false;
        for overlay in &surface.overlays {
            let GpuSurfaceOverlay::NativeHoverCursor {
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
        surface
            .overlays
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::NativeHoverCursor { .. }));
        surface.overlays.push(GpuSurfaceOverlay::NativeHoverCursor {
            ratio,
            color: cursor.color,
            width: cursor.width,
        });
        true
    }

    pub(super) fn clear_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let Some(surface) = gpu_surface_at_mut(&mut self.last_paint_plan.primitives, position)
        else {
            return false;
        };
        if surface.capabilities.native_hover_cursor.is_none() {
            return false;
        }
        let previous_len = surface.overlays.len();
        surface
            .overlays
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::NativeHoverCursor { .. }));
        previous_len != surface.overlays.len()
    }
}

fn gpu_surface_at_mut(
    primitives: &mut [PaintPrimitive],
    position: Point,
) -> Option<&mut PaintGpuSurface> {
    primitives.iter_mut().find_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface)
            if surface.rect.width() > 0.0
                && surface.rect.height() > 0.0
                && surface.content.is_renderable()
                && surface.rect.contains(position) =>
        {
            Some(surface)
        }
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{ImageRgba, Rect, Rgba8},
        runtime::{GpuHoverCursor, GpuSurfaceCapabilities, GpuSurfaceContent},
    };
    use std::sync::Arc;

    #[test]
    fn gpu_surface_lookup_skips_unrenderable_surface_content() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(40.0, 20.0));
        let capabilities = GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            native_hover_cursor: Some(GpuHoverCursor {
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            }),
        };
        let mut primitives = vec![
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
                    source_rect: rect,
                    atlas: Arc::new(ImageRgba::new(1, 1, vec![255; 4]).expect("valid image")),
                },
                capabilities,
                overlays: Vec::new(),
            }),
        ];

        let surface =
            gpu_surface_at_mut(&mut primitives, Point::new(10.0, 10.0)).expect("valid surface");

        assert_eq!(surface.key, 2);
    }

    #[test]
    fn gpu_surface_lookup_skips_empty_surface_rects() {
        let rect = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(0.0, 20.0));
        let mut primitives = vec![PaintPrimitive::GpuSurface(PaintGpuSurface {
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
                native_hover_cursor: None,
            },
            overlays: Vec::new(),
        })];

        assert!(gpu_surface_at_mut(&mut primitives, Point::new(0.0, 10.0)).is_none());
    }
}
