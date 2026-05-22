//! Pointer lifecycle helpers for the generic native Vello runner.

use super::*;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_cursor_moved(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        let Some(position) = logical_point_from_winit(position) else {
            self.last_cursor = None;
            return;
        };
        let previous = self.last_cursor;
        self.last_cursor = Some(position);
        if self.can_fast_path_native_hover_move(position) {
            self.update_gpu_surface_cursor_overlay(position);
            self.request_redraw_if_needed();
            return;
        }
        if previous.is_some_and(|previous| self.runtime_pointer_line_surface_contains(previous))
            && previous.is_some_and(|previous| self.clear_gpu_surface_cursor_overlay(previous))
        {
            self.request_redraw_if_needed();
            return;
        }
        let started = Instant::now();
        let outcome = self.core.route_pointer_move(position);
        maybe_log_route_profile("pointer_move", started.elapsed(), outcome);
        self.handle_gpu_surface_pointer_move_outcome(outcome, previous, position);
    }

    pub(super) fn handle_cursor_left(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(previous) = self.last_cursor
            && self.clear_gpu_surface_cursor_overlay(previous)
        {
            self.request_redraw_if_needed();
        }
        self.last_cursor = None;
        if self.core.runtime.hide_drag_preview_for_cursor_left() {
            self.rebuild_scene();
            self.request_redraw_if_needed();
            return;
        }
        let outcome = self.launch_external_drag_if_armed();
        self.handle_route_outcome(event_loop, outcome);
    }
}
