//! Pointer lifecycle helpers for the generic native Vello runner.

use super::frame_scheduler_policy::{
    ImmediateTransientCompletion, immediate_transient_completion_disposition,
};
use super::{
    FrameWork, FrameWorkReason, GenericNativeVelloRunner, GenericRouteOutcome, SceneRebuildMode,
    logical_point_from_winit, maybe_log_route_profile,
};
use crate::gui::input::InputTimestamp;
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use tracing::debug;
use winit::{dpi::PhysicalPosition, keyboard::ModifiersState};

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeCursorMovedRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) previous: Option<crate::gui::types::Point>,
    pub(super) position: Option<crate::gui::types::Point>,
    pub(super) apply_pointer_move_outcome: bool,
    pub(super) redraw_work: Option<FrameWork>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeCursorLeftRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) launch_external_drag: bool,
}

/// Publish the route accumulated while an ImmediateTransient ticket was live
/// only after its exact completion. The launch closure is deliberately called
/// here, after completion, so platform drag startup cannot precede the owner
/// fence or run after a completion mismatch.
pub(super) fn finalize_native_immediate_transient_route(
    completion: ImmediateTransientCompletion,
    mut routed: GenericRouteOutcome,
    launch_external_drag: bool,
    launch: impl FnOnce() -> GenericRouteOutcome,
) -> Option<GenericRouteOutcome> {
    let disposition = immediate_transient_completion_disposition(completion)?;
    if launch_external_drag {
        routed.merge(launch());
    }
    Some(routed.with_native_input_stage_disposition(disposition))
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_cursor_entered(&mut self) {
        self.set_native_cursor_visible(true);
        self.force_native_cursor(crate::widgets::WidgetCursor::Default);
    }

    #[cfg(test)]
    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let route = self.route_cursor_moved_with_timestamp(position, InputTimestamp::capture());
        self.apply_cursor_moved_route(route);
    }

    pub(super) fn route_cursor_moved_with_timestamp(
        &mut self,
        position: PhysicalPosition<f64>,
        timestamp: InputTimestamp,
    ) -> NativeCursorMovedRoute {
        let timestamp = Some(timestamp);
        let Some(position) = logical_point_from_winit(position, self.window.dpi_scale) else {
            self.input.last_cursor = None;
            self.set_native_cursor_visible(true);
            self.force_native_cursor(crate::widgets::WidgetCursor::Default);
            return NativeCursorMovedRoute {
                outcome: GenericRouteOutcome::default(),
                previous: self.input.last_cursor,
                position: None,
                apply_pointer_move_outcome: false,
                redraw_work: None,
            };
        };
        let sequence_range = self.input.input_sequence_allocator.allocate();
        let previous = self.input.last_cursor;
        self.input.last_cursor = Some(position);
        self.core.set_current_pointer_position(Some(position));
        let modifiers = self.pointer_modifiers();
        if previous.is_none() {
            self.force_native_cursor(crate::widgets::WidgetCursor::Default);
        }
        if self.core.runtime.scrollbar_drag_active() {
            if self.pending_interactive_scroll_flush_is_due(Instant::now()) {
                let outcome = self.core.route_pointer_move_with_metadata(
                    position,
                    modifiers,
                    timestamp,
                    sequence_range,
                );
                return NativeCursorMovedRoute {
                    outcome,
                    previous,
                    position: Some(position),
                    apply_pointer_move_outcome: true,
                    redraw_work: None,
                };
            }
            self.queue_scrollbar_drag_with_metadata_for_immediate_transient(
                position,
                modifiers,
                timestamp,
                sequence_range,
            );
            return NativeCursorMovedRoute {
                outcome: GenericRouteOutcome::default(),
                previous,
                position: Some(position),
                apply_pointer_move_outcome: false,
                redraw_work: Some(FrameWork::None),
            };
        }
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
            self.update_native_cursor_at_last_position();
            return NativeCursorMovedRoute {
                outcome: GenericRouteOutcome::default(),
                previous,
                position: Some(position),
                apply_pointer_move_outcome: false,
                redraw_work: Some(FrameWork::PaintOnly {
                    reason: FrameWorkReason::PointerHover,
                }),
            };
        }
        let cleared_previous_gpu_hover = previous
            .is_some_and(|previous| self.runtime_pointer_line_surface_contains(previous))
            && previous.is_some_and(|previous| self.clear_gpu_surface_cursor_overlay(previous));
        if cleared_previous_gpu_hover {
            self.update_native_cursor_at_last_position();
        }
        let started = Instant::now();
        let outcome = self.core.route_pointer_move_with_metadata(
            position,
            modifiers,
            timestamp,
            sequence_range,
        );
        if self.core.runtime.pointer_capture().is_none() {
            self.update_native_cursor_at_last_position();
        }
        maybe_log_route_profile("pointer_move", started.elapsed(), outcome);
        NativeCursorMovedRoute {
            outcome,
            previous,
            position: Some(position),
            apply_pointer_move_outcome: true,
            redraw_work: cleared_previous_gpu_hover.then_some(FrameWork::PaintOnly {
                reason: FrameWorkReason::NativePointerClear,
            }),
        }
    }

    pub(super) fn apply_cursor_moved_route(&mut self, route: NativeCursorMovedRoute) {
        if let Some(work) = route.redraw_work {
            if matches!(
                route.outcome.native_input_stage_disposition(),
                Some(
                    super::frame_scheduler_policy::NativeInputStageDisposition::DeferLowerPriority
                )
            ) {
                let mut deferred = GenericRouteOutcome::default();
                deferred.request_frame_work(work);
                self.defer_lower_priority_route_outcome(deferred);
                self.request_redraw_for_deferred_frame_work(work);
            } else {
                self.request_redraw_for_frame_work(work);
            }
        }
        if route.apply_pointer_move_outcome
            && let Some(position) = route.position
        {
            self.handle_gpu_surface_pointer_move_outcome(route.outcome, route.previous, position);
        }
    }

    pub(super) fn route_cursor_left(&mut self) -> NativeCursorLeftRoute {
        let external_drag_armed_before_clear = self.core.runtime.external_drag_armed();
        if external_drag_armed_before_clear {
            debug!(
                target: "radiant::external_drag",
                event = "external_drag.pointer_exited",
                "Pointer exited with external drag armed"
            );
        }
        let pointer_cleared = self.clear_native_pointer_presence();
        let mut outcome = pointer_cleared;
        let preview_hidden = self.core.runtime.hide_drag_preview_for_cursor_left();
        let launch_external_drag = self.core.runtime.external_drag_armed();
        if preview_hidden && !launch_external_drag {
            outcome.request_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::ExternalDragPreview,
                mode: SceneRebuildMode::Immediate,
            });
        }
        NativeCursorLeftRoute {
            outcome,
            launch_external_drag,
        }
    }

    pub(super) fn handle_focus_lost_before_external_drag(&mut self) -> GenericRouteOutcome {
        self.window.native_focus_lost = true;
        self.input.tab_sequence_latch = None;
        self.input.effective_pointer_gesture = None;
        let mut outcome = self.clear_native_pointer_presence();
        outcome.merge(self.clear_native_modifier_state());
        outcome.merge(self.core.route_focus_lost());
        outcome
    }

    pub(super) fn handle_focus_regained_after_native_modal_loop(&mut self) -> GenericRouteOutcome {
        self.input.tab_sequence_latch = None;
        self.clear_native_visual_request_wake_timing();
        // Record normal-window activation before the exact transient ticket is
        // completed. Native visibility and visual-mailbox work are applied by
        // the lifecycle caller only after that completion fence holds.
        self.record_normal_window_activation_intent("focus-regained");
        let mut outcome = GenericRouteOutcome::default();
        outcome.request_frame_work(FrameWork::PaintOnly {
            reason: FrameWorkReason::NativeFocusRegained,
        });
        if !std::mem::take(&mut self.window.native_focus_lost) {
            return outcome;
        }
        let command = self.core.runtime.host_native_focus_regained();
        let command_outcome = self.core.runtime.execute_command(command);
        outcome.merge(self.core.route_command_outcome(command_outcome));
        outcome
    }

    fn clear_native_pointer_presence(&mut self) -> GenericRouteOutcome {
        let mut outcome = GenericRouteOutcome::default();
        if let Some(previous) = self.input.last_cursor
            && self.clear_gpu_surface_cursor_overlay(previous)
        {
            outcome.request_scene_rebuild(FrameWorkReason::NativePointerClear);
        }
        self.input.pending_scrollbar_drag = None;
        self.core.set_current_pointer_position(None);
        if self.core.runtime.clear_pointer_hover() {
            outcome.request_scene_rebuild(FrameWorkReason::NativePointerClear);
        }
        self.input.last_cursor = None;
        self.set_native_cursor_visible(true);
        self.set_native_cursor(crate::widgets::WidgetCursor::Default);
        outcome
    }

    fn clear_native_modifier_state(&mut self) -> GenericRouteOutcome {
        self.input.last_navigation_key_repeat = None;
        self.input.pending_gpu_surface_wheel = None;
        self.input.pending_scroll_container_wheel = None;
        self.input.pending_scrollbar_drag = None;
        if self.input.modifiers.is_empty() {
            return GenericRouteOutcome::default();
        }
        self.input.modifiers = ModifiersState::default();
        self.core
            .route_pointer_modifiers_changed(crate::widgets::PointerModifiers::default(), None)
    }
}
