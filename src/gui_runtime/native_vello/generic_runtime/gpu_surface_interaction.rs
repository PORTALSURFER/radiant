//! Interaction fast paths for retained GPU surface primitives.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome,
    gpu_surface_cursor::{
        clear_surface_cursor_overlay, topmost_native_hover_surface_index,
        update_surface_cursor_overlay,
    },
};
use crate::{
    gui::types::{Point, Vector2},
    runtime::PaintPrimitive,
};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn handle_gpu_surface_route_outcome(
        &mut self,
        mut outcome: GenericRouteOutcome,
        position: Point,
        delta: Vector2,
    ) {
        self.merge_due_timed_frame_for_route(&mut outcome);
        if !outcome.needs_redraw() {
            return;
        }
        if outcome.interactive_scene_rebuild_requested {
            self.defer_interactive_scene_rebuild();
            self.request_redraw_if_needed();
            return;
        }
        if self.can_fast_path_gpu_surface_route(position, delta) {
            self.timing.deferred_surface_refresh = true;
            self.request_redraw_if_needed();
            return;
        }
        self.rebuild_scene();
        self.request_redraw_if_needed();
    }

    pub(super) fn handle_gpu_surface_pointer_move_outcome(
        &mut self,
        mut outcome: GenericRouteOutcome,
        previous: Option<Point>,
        position: Point,
    ) {
        self.merge_due_timed_frame_for_route(&mut outcome);
        if !outcome.needs_redraw() {
            return;
        }
        if outcome.paint_only_requested && !outcome.needs_scene_rebuild() {
            if outcome.deferred_surface_refresh_requested {
                self.timing.deferred_surface_refresh = true;
            }
            self.request_redraw_if_needed();
            return;
        }
        if outcome.deferred_surface_refresh_requested && !outcome.needs_scene_rebuild() {
            self.timing.deferred_surface_refresh = true;
            self.request_redraw_if_needed();
            return;
        }
        if outcome.routed {
            if outcome.interactive_scene_rebuild_requested {
                let now = std::time::Instant::now();
                if self.should_rebuild_interactive_scene_now(now) {
                    if outcome.interactive_surface_refresh_requested {
                        self.refresh_and_rebuild_scene_for_interactive_route_now();
                    } else {
                        self.rebuild_scene_for_interactive_route_now();
                    }
                } else {
                    self.defer_interactive_scene_rebuild();
                }
            } else {
                self.rebuild_scene();
            }
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

    pub(super) fn can_fast_path_gpu_surface_pointer_move(
        &self,
        previous: Option<Point>,
        position: Point,
    ) -> bool {
        if self.core.runtime.pointer_capture().is_some() || self.core.runtime.drag_session_active()
        {
            return false;
        }
        let Some(previous) = previous else {
            return false;
        };
        if !self.gpu_surface_fast_path_allows_top_pointer_target(position) {
            return false;
        }
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| {
                region.fast_pointer_move && region.contains(previous) && region.contains(position)
            })
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
            && !self.core.runtime.drag_session_active()
            && self.runtime_pointer_line_surface_contains(position)
            && self.gpu_surface_fast_path_allows_top_pointer_target(position)
    }

    fn gpu_surface_fast_path_allows_top_pointer_target(&self, position: Point) -> bool {
        let Some(widget_id) = self.core.runtime.widget_at(position) else {
            return true;
        };
        self.topmost_gpu_surface_interaction_region_at(position)
            .is_some_and(|region| region.widget_id == widget_id)
    }

    fn topmost_gpu_surface_interaction_region_at(
        &self,
        position: Point,
    ) -> Option<super::GpuSurfaceInteractionRegion> {
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .rev()
            .copied()
            .find(|region| region.contains(position))
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
        let mut changed = false;
        for surface in self
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
        {
            changed = clear_surface_cursor_overlay(surface) || changed;
        }
        if changed {
            self.frame.mark_composited_base_dirty();
        }
        changed
    }
}
