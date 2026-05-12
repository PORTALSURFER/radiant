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
            let GpuSurfaceOverlay::VerticalCursor {
                ratio: current_ratio,
                color,
                width,
            } = overlay;
            cursor_count += 1;
            cursor_is_current |=
                *current_ratio == ratio && *color == cursor.color && *width == cursor.width;
        }
        if cursor_count == 1 && cursor_is_current {
            return false;
        }
        surface
            .overlays
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::VerticalCursor { .. }));
        surface.overlays.push(GpuSurfaceOverlay::VerticalCursor {
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
            .retain(|overlay| !matches!(overlay, GpuSurfaceOverlay::VerticalCursor { .. }));
        previous_len != surface.overlays.len()
    }
}

fn gpu_surface_at_mut(
    primitives: &mut [PaintPrimitive],
    position: Point,
) -> Option<&mut PaintGpuSurface> {
    primitives.iter_mut().find_map(|primitive| match primitive {
        PaintPrimitive::GpuSurface(surface) if surface.rect.contains(position) => Some(surface),
        _ => None,
    })
}
