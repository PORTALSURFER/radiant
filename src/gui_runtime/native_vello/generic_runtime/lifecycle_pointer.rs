//! Pointer lifecycle helpers for the generic native Vello runner.

use super::{GenericNativeVelloRunner, logical_point_from_winit, maybe_log_route_profile};
use crate::runtime::RuntimeBridge;
use std::time::Instant;
use winit::{dpi::PhysicalPosition, event_loop::ActiveEventLoop};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        let Some(position) = logical_point_from_winit(position) else {
            self.input.last_cursor = None;
            return;
        };
        let previous = self.input.last_cursor;
        self.input.last_cursor = Some(position);
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
        if let Some(previous) = self.input.last_cursor
            && self.clear_gpu_surface_cursor_overlay(previous)
        {
            self.request_redraw_if_needed();
        }
        self.input.last_cursor = None;
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
}
