//! Wheel coalescing fast paths for retained GPU surface primitives.

use super::{GenericNativeVelloRunner, RenderFrameProfile, maybe_log_route_profile};
use crate::gui::types::{Point, Vector2};
use crate::widgets::PointerModifiers;

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingGpuSurfaceWheel {
    pub(super) position: Point,
    pub(super) delta: Vector2,
    pub(super) modifiers: PointerModifiers,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingScrollbarDrag {
    pub(super) position: Point,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn queue_gpu_surface_wheel(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) {
        match &mut self.input.pending_gpu_surface_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
                pending.modifiers = modifiers;
            }
            None => {
                self.input.pending_gpu_surface_wheel = Some(PendingGpuSurfaceWheel {
                    position,
                    delta,
                    modifiers,
                });
            }
        }
        self.update_gpu_surface_cursor_overlay(position);
        self.request_redraw_if_needed();
    }

    pub(super) fn queue_scroll_container_wheel(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) {
        match &mut self.input.pending_scroll_container_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
                pending.modifiers = modifiers;
            }
            None => {
                self.input.pending_scroll_container_wheel = Some(PendingGpuSurfaceWheel {
                    position,
                    delta,
                    modifiers,
                });
            }
        }
        self.request_redraw_if_needed();
    }

    pub(super) fn queue_scrollbar_drag(&mut self, position: Point) {
        self.input.pending_scrollbar_drag = Some(PendingScrollbarDrag { position });
        self.request_redraw_if_needed();
    }

    pub(super) fn flush_pending_scrollbar_drag_now(&mut self) {
        let Some(pending) = self.input.pending_scrollbar_drag.take() else {
            return;
        };
        let outcome = self.core.route_pointer_move(pending.position);
        maybe_log_route_profile(
            "coalesced_scrollbar_drag",
            std::time::Duration::ZERO,
            outcome,
        );
        self.handle_gpu_surface_pointer_move_outcome(
            outcome,
            Some(pending.position),
            pending.position,
        );
    }

    pub(super) fn flush_pending_wheel_input_now(&mut self) {
        let mut profile = RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.flush_pending_scroll_container_wheel(&mut profile);
    }

    pub(super) fn flush_pending_gpu_surface_wheel(&mut self, profile: &mut RenderFrameProfile) {
        let Some(pending) = self.input.pending_gpu_surface_wheel.take() else {
            return;
        };
        let (outcome, elapsed) = profile.measure(|| {
            self.core.route_scroll_deferred_refresh_with_modifiers(
                pending.position,
                pending.delta,
                pending.modifiers,
            )
        });
        profile.coalesced_wheel_route = elapsed;
        maybe_log_route_profile("coalesced_wheel", profile.coalesced_wheel_route, outcome);
        if outcome.interactive_surface_refresh_requested {
            self.refresh_and_rebuild_scene_for_interactive_route_now();
            return;
        }
        if outcome.interactive_scene_rebuild_requested {
            self.rebuild_scene_for_interactive_route_now();
            return;
        }
        if outcome.needs_redraw() {
            self.timing.deferred_surface_refresh = true;
        }
    }

    pub(super) fn flush_pending_scroll_container_wheel(
        &mut self,
        profile: &mut RenderFrameProfile,
    ) {
        let Some(pending) = self.input.pending_scroll_container_wheel.take() else {
            return;
        };
        let (outcome, elapsed) = profile.measure(|| {
            self.core.route_scroll_deferred_refresh_with_modifiers(
                pending.position,
                pending.delta,
                pending.modifiers,
            )
        });
        profile.coalesced_wheel_route += elapsed;
        maybe_log_route_profile("coalesced_scroll_wheel", elapsed, outcome);
        if outcome.interactive_surface_refresh_requested {
            self.refresh_and_rebuild_scene_for_interactive_route_now();
            return;
        }
        if outcome.interactive_scene_rebuild_requested {
            self.rebuild_scene_for_interactive_route_now();
            return;
        }
        if outcome.deferred_surface_refresh_requested {
            self.timing.deferred_surface_refresh = true;
        }
        if outcome.needs_scene_rebuild() {
            self.rebuild_scene_for_interactive_route_now();
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

    pub(super) fn can_coalesce_scroll_container_wheel(
        &self,
        position: Point,
        delta: Vector2,
    ) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical
            && !self.core.runtime.wheel_widget_accepts_at(position)
            && self
                .core
                .runtime
                .scroll_container_accepts_wheel_at(position)
    }
}
