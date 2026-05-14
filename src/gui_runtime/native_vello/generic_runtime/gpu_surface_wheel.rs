//! Wheel coalescing fast paths for retained GPU surface primitives.

use super::{GenericNativeVelloRunner, RenderFrameProfile, maybe_log_route_profile};
use crate::gui::types::{Point, Vector2};
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

    pub(super) fn can_fast_path_gpu_surface_route(&self, position: Point, delta: Vector2) -> bool {
        let is_horizontal_pan = delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON;
        !is_horizontal_pan && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn paint_plan_has_coalescing_gpu_surface_at(&self, position: Point) -> bool {
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| region.coalesce_vertical_wheel && region.contains(position))
    }

    pub(super) fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }
}
