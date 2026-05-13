//! Interaction fast paths for retained GPU surface primitives.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, RenderFrameProfile,
    gpu_surface_cursor::{
        clear_surface_cursor_overlay, topmost_native_hover_surface_index,
        update_surface_cursor_overlay,
    },
    maybe_log_route_profile,
};
use crate::{
    gui::types::{Point, Vector2},
    runtime::PaintPrimitive,
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
        if outcome.paint_only_requested && !outcome.needs_scene_rebuild() {
            self.request_redraw_if_needed();
            return;
        }
        if outcome.routed {
            self.rebuild_scene();
            self.request_redraw_if_needed();
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
        if self.core.runtime.pointer_capture().is_some() {
            return false;
        }
        let Some(previous) = previous else {
            return false;
        };
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| {
                region.fast_pointer_move && region.contains(previous) && region.contains(position)
            })
    }

    pub(super) fn paint_plan_has_coalescing_gpu_surface_at(&self, position: Point) -> bool {
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| region.coalesce_vertical_wheel && region.contains(position))
    }

    pub(super) fn runtime_pointer_line_surface_contains(&self, position: Point) -> bool {
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| {
                region.runtime_overlays.pointer_vertical_line.is_some() && region.contains(position)
            })
    }

    pub(super) fn can_fast_path_native_hover_move(&self, position: Point) -> bool {
        self.core.runtime.pointer_capture().is_none()
            && self.runtime_pointer_line_surface_contains(position)
    }

    pub(super) fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical && self.paint_plan_has_coalescing_gpu_surface_at(position)
    }

    pub(super) fn update_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let target_index =
            topmost_native_hover_surface_index(&self.frame.last_paint_plan.primitives, position);
        let mut changed = false;
        for (index, primitive) in self.frame.last_paint_plan.primitives.iter_mut().enumerate() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if surface
                .capabilities
                .runtime_overlays
                .pointer_vertical_line
                .is_none()
            {
                continue;
            }
            if Some(index) == target_index {
                changed |= update_surface_cursor_overlay(surface, position);
            } else {
                changed |= clear_surface_cursor_overlay(surface);
            }
        }
        if changed {
            self.frame.mark_composited_base_dirty();
        }
        changed
    }

    pub(super) fn clear_gpu_surface_cursor_overlay(&mut self, position: Point) -> bool {
        let changed = self
            .frame
            .last_paint_plan
            .primitives
            .iter_mut()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::GpuSurface(surface)
                    if surface
                        .capabilities
                        .runtime_overlays
                        .pointer_vertical_line
                        .is_some()
                        && surface.rect.contains(position) =>
                {
                    Some(surface)
                }
                _ => None,
            })
            .fold(false, |changed, surface| {
                clear_surface_cursor_overlay(surface) || changed
            });
        if changed {
            self.frame.mark_composited_base_dirty();
        }
        changed
    }
}
