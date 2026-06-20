//! Pointer lifecycle helpers for the generic native Vello runner.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, logical_point_from_winit,
    maybe_log_route_profile,
};
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop, keyboard::ModifiersState};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_cursor_entered(&mut self) {
        self.set_native_cursor_visible(true);
        self.force_native_cursor(crate::widgets::WidgetCursor::Default);
    }

    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let Some(position) = logical_point_from_winit(position, self.window.dpi_scale) else {
            self.input.last_cursor = None;
            self.set_native_cursor_visible(true);
            self.force_native_cursor(crate::widgets::WidgetCursor::Default);
            return;
        };
        let previous = self.input.last_cursor;
        self.input.last_cursor = Some(position);
        if previous.is_none() {
            self.force_native_cursor(crate::widgets::WidgetCursor::Default);
        }
        if self.core.runtime.scrollbar_drag_active() {
            if self.pending_interactive_scroll_flush_is_due(Instant::now()) {
                let outcome = self.core.route_pointer_move(position);
                self.handle_gpu_surface_pointer_move_outcome(outcome, previous, position);
                return;
            }
            self.queue_scrollbar_drag(position);
            return;
        }
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
            self.update_native_cursor_at_last_position();
            self.request_redraw_if_needed();
            return;
        }
        let cleared_previous_gpu_hover = previous
            .is_some_and(|previous| self.runtime_pointer_line_surface_contains(previous))
            && previous.is_some_and(|previous| self.clear_gpu_surface_cursor_overlay(previous));
        if cleared_previous_gpu_hover {
            self.update_native_cursor_at_last_position();
            self.request_redraw_if_needed();
        }
        let started = Instant::now();
        let outcome = self.core.route_pointer_move(position);
        if self.core.runtime.pointer_capture().is_none() {
            self.update_native_cursor_at_last_position();
        }
        maybe_log_route_profile("pointer_move", started.elapsed(), outcome);
        self.handle_gpu_surface_pointer_move_outcome(outcome, previous, position);
    }

    pub(super) fn handle_cursor_left(&mut self, event_loop: &ActiveEventLoop) {
        let pointer_cleared = self.clear_native_pointer_presence();
        if pointer_cleared.repaint_requested {
            self.request_redraw_if_needed();
        }
        let preview_hidden = self.core.runtime.hide_drag_preview_for_cursor_left();
        if preview_hidden {
            if self.core.runtime.external_drag_armed() {
                let outcome = self.launch_external_drag_if_armed();
                self.handle_route_outcome(event_loop, outcome);
            } else {
                self.rebuild_scene();
                self.request_redraw_if_needed();
            }
            return;
        }
        let outcome = self.launch_external_drag_if_armed();
        self.handle_route_outcome(event_loop, outcome);
    }

    pub(super) fn handle_focus_lost_before_external_drag(&mut self) -> GenericRouteOutcome {
        let mut outcome = self.clear_native_pointer_presence();
        outcome.merge(self.clear_native_modifier_state());
        outcome.merge(self.core.route_focus_lost());
        outcome
    }

    pub(super) fn handle_focus_regained_after_native_modal_loop(&mut self) {
        self.timing.redraw_requested = false;
        self.request_redraw_if_needed();
    }

    fn clear_native_pointer_presence(&mut self) -> GenericRouteOutcome {
        let mut outcome = GenericRouteOutcome::default();
        if let Some(previous) = self.input.last_cursor
            && self.clear_gpu_surface_cursor_overlay(previous)
        {
            outcome.repaint_requested = true;
        }
        self.input.pending_scrollbar_drag = None;
        if self.core.runtime.clear_pointer_hover() {
            outcome.repaint_requested = true;
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
            .route_pointer_modifiers_changed(crate::widgets::PointerModifiers::default())
    }
}
