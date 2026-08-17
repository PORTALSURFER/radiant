//! Wheel coalescing fast paths for retained GPU surface primitives.

use super::{FrameWork, GenericNativeVelloRunner, RenderFrameProfile, maybe_log_route_profile};
use crate::gui::input::{InputSequenceRange, InputTimestamp};
use crate::gui::types::{Point, Vector2};
use crate::widgets::PointerModifiers;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GpuSurfaceWheelAxis {
    Horizontal,
    Vertical,
}

impl GpuSurfaceWheelAxis {
    fn from_delta(delta: Vector2) -> Self {
        if delta.x.abs() > delta.y.abs() {
            Self::Horizontal
        } else {
            Self::Vertical
        }
    }

    fn semantic_delta(self, delta: Vector2) -> Vector2 {
        match self {
            Self::Horizontal => Vector2::new(delta.x, 0.0),
            Self::Vertical => Vector2::new(0.0, delta.y),
        }
    }
}

fn extend_pending_sequence_range(
    current: &mut Option<InputSequenceRange>,
    incoming: Option<InputSequenceRange>,
) {
    match (*current, incoming) {
        (Some(mut current_range), Some(incoming_range)) => {
            current_range.extend_end(incoming_range.end());
            *current = Some(current_range);
        }
        _ => *current = None,
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingGpuSurfaceWheel {
    pub(super) position: Point,
    pub(super) delta: Vector2,
    pub(super) modifiers: PointerModifiers,
    pub(super) timestamp: Option<InputTimestamp>,
    pub(super) sequence_range: Option<InputSequenceRange>,
    axis: GpuSurfaceWheelAxis,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PendingScrollbarDrag {
    pub(super) position: Point,
    pub(super) modifiers: PointerModifiers,
    pub(super) timestamp: Option<InputTimestamp>,
    pub(super) sequence_range: Option<InputSequenceRange>,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    #[cfg(test)]
    pub(super) fn queue_gpu_surface_wheel(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) {
        self.queue_gpu_surface_wheel_with_timestamp(position, delta, modifiers, None);
    }

    #[cfg(test)]
    pub(super) fn queue_gpu_surface_wheel_with_timestamp(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) {
        self.queue_gpu_surface_wheel_with_metadata(position, delta, modifiers, timestamp, None);
    }

    pub(super) fn queue_gpu_surface_wheel_with_metadata(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) {
        let axis = GpuSurfaceWheelAxis::from_delta(delta);
        let delta = axis.semantic_delta(delta);
        if self
            .input
            .pending_gpu_surface_wheel
            .as_ref()
            .is_some_and(|pending| pending.axis != axis)
        {
            self.flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());
        }
        match &mut self.input.pending_gpu_surface_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
                pending.modifiers = modifiers;
                pending.timestamp = timestamp;
                extend_pending_sequence_range(&mut pending.sequence_range, sequence_range);
            }
            None => {
                self.input.pending_gpu_surface_wheel = Some(PendingGpuSurfaceWheel {
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                    axis,
                });
            }
        }
        self.update_gpu_surface_cursor_overlay(position);
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    pub(super) fn queue_scroll_container_wheel_with_metadata(
        &mut self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) {
        let axis = GpuSurfaceWheelAxis::from_delta(delta);
        let delta = axis.semantic_delta(delta);
        if self
            .input
            .pending_scroll_container_wheel
            .as_ref()
            .is_some_and(|pending| pending.axis != axis)
        {
            self.flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());
        }
        match &mut self.input.pending_scroll_container_wheel {
            Some(pending) => {
                pending.position = position;
                pending.delta = Vector2::new(pending.delta.x + delta.x, pending.delta.y + delta.y);
                pending.modifiers = modifiers;
                pending.timestamp = timestamp;
                extend_pending_sequence_range(&mut pending.sequence_range, sequence_range);
            }
            None => {
                self.input.pending_scroll_container_wheel = Some(PendingGpuSurfaceWheel {
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                    axis,
                });
            }
        }
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    #[cfg(test)]
    pub(super) fn queue_scrollbar_drag(&mut self, position: Point) {
        self.queue_scrollbar_drag_with_metadata(position, PointerModifiers::default(), None, None);
    }

    pub(super) fn queue_scrollbar_drag_with_metadata(
        &mut self,
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) {
        match &mut self.input.pending_scrollbar_drag {
            Some(pending) => {
                pending.position = position;
                pending.modifiers = modifiers;
                pending.timestamp = timestamp;
                extend_pending_sequence_range(&mut pending.sequence_range, sequence_range);
            }
            None => {
                self.input.pending_scrollbar_drag = Some(PendingScrollbarDrag {
                    position,
                    modifiers,
                    timestamp,
                    sequence_range,
                });
            }
        }
        self.request_redraw_for_frame_work(FrameWork::None);
    }

    pub(super) fn flush_pending_scrollbar_drag_now(&mut self) {
        let Some(pending) = self.input.pending_scrollbar_drag.take() else {
            return;
        };
        let outcome = self.core.route_pointer_move_with_metadata(
            pending.position,
            pending.modifiers,
            pending.timestamp,
            pending.sequence_range,
        );
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
            self.core.route_scroll_deferred_refresh_with_metadata(
                pending.position,
                pending.delta,
                pending.modifiers,
                pending.timestamp,
                pending.sequence_range,
            )
        });
        profile.coalesced_wheel_route = elapsed;
        maybe_log_route_profile("coalesced_wheel", profile.coalesced_wheel_route, outcome);
        self.record_frame_work(outcome.frame_work());
        if outcome.is_interactive_surface_refresh() {
            self.refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
                outcome.surface_refresh_scope_or_surface(),
            );
            return;
        }
        if outcome.is_interactive_scene_rebuild() {
            self.rebuild_scene_for_interactive_route_now();
            return;
        }
        if outcome.needs_scene_rebuild() {
            if matches!(
                outcome.frame_work(),
                FrameWork::RebuildScene {
                    mode: super::SceneRebuildMode::ImmediateWithSurfaceRefresh,
                    ..
                }
            ) {
                self.refresh_and_rebuild_scene_now_with_scope(
                    outcome.surface_refresh_scope_or_surface(),
                );
            } else {
                self.rebuild_scene();
            }
            return;
        }
        if outcome.is_deferred_surface_refresh() {
            self.defer_surface_refresh_with_scope(outcome.surface_refresh_scope_or_surface());
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
            self.core.route_scroll_deferred_refresh_with_metadata(
                pending.position,
                pending.delta,
                pending.modifiers,
                pending.timestamp,
                pending.sequence_range,
            )
        });
        profile.coalesced_wheel_route += elapsed;
        maybe_log_route_profile("coalesced_scroll_wheel", elapsed, outcome);
        self.record_frame_work(outcome.frame_work());
        if outcome.is_interactive_surface_refresh() {
            self.refresh_and_rebuild_scene_for_interactive_route_now_with_scope(
                outcome.surface_refresh_scope_or_surface(),
            );
            self.refresh_pointer_hover_after_scroll();
            return;
        }
        if outcome.is_interactive_scene_rebuild() {
            self.rebuild_scene_for_interactive_route_now();
            self.refresh_pointer_hover_after_scroll();
            return;
        }
        if outcome.is_deferred_surface_refresh() {
            self.defer_surface_refresh_with_scope(outcome.surface_refresh_scope_or_surface());
        }
        if outcome.needs_scene_rebuild() {
            self.rebuild_scene_for_interactive_route_now();
            self.refresh_pointer_hover_after_scroll();
        }
    }

    /// Re-hit-test the native pointer after a coalesced scroll commits a new
    /// materialized layout. Pointer motion can arrive while the wheel is still
    /// pending, so the interaction's retained hover target may refer to a row
    /// that is no longer in the current virtual window.
    fn refresh_pointer_hover_after_scroll(&mut self) {
        let Some(position) = self.input.last_cursor else {
            return;
        };
        let outcome = self.core.route_pointer_move(position);
        self.handle_gpu_surface_pointer_move_outcome(outcome, Some(position), position);
    }

    pub(super) fn can_fast_path_gpu_surface_route(&self, position: Point, delta: Vector2) -> bool {
        self.can_coalesce_gpu_surface_wheel(position, delta)
    }

    pub(super) fn paint_plan_has_coalescing_gpu_surface_at(
        &self,
        position: Point,
        delta: Vector2,
    ) -> bool {
        let axis = GpuSurfaceWheelAxis::from_delta(delta);
        self.frame
            .gpu_surface_interaction_regions
            .iter()
            .any(|region| {
                region.contains(position)
                    && if axis == GpuSurfaceWheelAxis::Horizontal {
                        region.coalesce_horizontal_wheel
                    } else {
                        region.coalesce_vertical_wheel
                    }
            })
    }

    pub(super) fn can_coalesce_gpu_surface_wheel(&self, position: Point, delta: Vector2) -> bool {
        let has_delta = delta.x.abs().max(delta.y.abs()) > f32::EPSILON;
        has_delta && self.paint_plan_has_coalescing_gpu_surface_at(position, delta)
    }

    #[cfg(test)]
    pub(super) fn can_coalesce_scroll_container_wheel(
        &self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
    ) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical
            && !self
                .core
                .runtime
                .wheel_widget_accepts_at(position, delta, modifiers)
            && self
                .core
                .runtime
                .scroll_container_accepts_wheel_at(position)
    }

    pub(super) fn can_coalesce_scroll_container_wheel_with_timestamp(
        &self,
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> bool {
        let is_vertical = delta.y.abs() >= delta.x.abs() && delta.y.abs() > f32::EPSILON;
        is_vertical
            && !self
                .core
                .runtime
                .wheel_widget_accepts_at_with_metadata(position, delta, modifiers, timestamp)
            && self
                .core
                .runtime
                .scroll_container_accepts_wheel_at(position)
    }
}
