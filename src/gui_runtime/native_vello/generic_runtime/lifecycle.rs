//! Winit application lifecycle for the generic native Vello runner.

use super::{
    AuxiliaryWindowEventResult, GenericNativeVelloRunner, RuntimeUserEvent, TimedFrameCadence,
    animation_frame_interval, should_start_popup_window_drag, timed_frame_cadence,
    timed_frame_target_fps,
};
use crate::runtime::RuntimeBridge;
use std::time::{Duration, Instant};
use tracing::warn;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

const LATE_TIMED_FRAME_LOG_THRESHOLD: Duration = Duration::from_millis(24);
const LATE_TIMED_FRAME_MAX_CONTINUOUS_GAP: Duration = Duration::from_secs(1);

impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>
    for GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.window.is_none() {
            self.initialize_runtime(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window.id {
            let Some(index) = self
                .auxiliary_windows
                .iter()
                .position(|window| window.window_id() == Some(window_id))
            else {
                return;
            };
            let AuxiliaryWindowEventResult { closed, messages } =
                self.auxiliary_windows[index].route_window_event(event_loop, event);
            if closed {
                self.auxiliary_windows.remove(index);
            }
            if !messages.is_empty() {
                self.dispatch_auxiliary_messages(event_loop, messages);
            }
            return;
        }
        match event {
            WindowEvent::CloseRequested if self.core.runtime.bridge_mut().close_requested() => {
                event_loop.exit();
            }
            WindowEvent::CloseRequested => {}
            WindowEvent::Resized(size) => self.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::Focused(false) => {
                let routed = self.handle_focus_lost_before_external_drag();
                self.handle_route_outcome(event_loop, routed);
                if self.core.runtime.external_drag_armed() {
                    let outcome = self.launch_external_drag_if_armed();
                    self.handle_route_outcome(event_loop, outcome);
                }
            }
            WindowEvent::Focused(true) => self.handle_focus_regained_after_native_modal_loop(),
            WindowEvent::CursorEntered { .. } => self.handle_cursor_entered(),
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_cursor_moved(position);
            }
            WindowEvent::HoveredFile(path) => self.handle_native_file_hover(event_loop, path),
            WindowEvent::HoveredFileCancelled => self.handle_native_file_cancel(event_loop),
            WindowEvent::DroppedFile(path) => self.handle_native_file_drop(event_loop, path),
            WindowEvent::CursorLeft { .. } => self.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                let route = self.route_native_mouse_input(button, state);
                if route.is_pressed()
                    && let (Some(position), Some(button)) = (route.position, route.button)
                    && should_start_popup_window_drag(
                        &self.options,
                        position,
                        button,
                        route.outcome.routed,
                    )
                    && let Some(window) = self.window.window.as_ref()
                    && let Err(err) = window.drag_window()
                {
                    warn!("radiant generic native vello: popup window drag failed: {err}");
                }
                self.handle_route_outcome(event_loop, route.outcome);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let route = self.route_native_mouse_wheel(delta);
                self.handle_route_outcome(event_loop, route.outcome);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                if self.should_launch_external_drag_before_app_switch(state) {
                    self.input.modifiers = state;
                    let outcome = self.launch_external_drag_if_armed();
                    self.handle_route_outcome(event_loop, outcome);
                } else {
                    let routed = self.route_native_modifiers_changed(state);
                    self.handle_route_outcome(event_loop, routed);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                self.runtime_wakeup.clear_pending();
                let outcome = self.core.drain_runtime_messages();
                self.handle_route_outcome(event_loop, outcome);
            }
            RuntimeUserEvent::OpenFiles(paths) => self.handle_native_file_open(event_loop, paths),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.window.is_none() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        let animation_activity = self.core.animation_activity();
        let now = Instant::now();
        let needs_text_caret_animation = self.core.has_focused_text_input();
        let frame_target_fps = timed_frame_target_fps(
            self.options.normalized_target_fps(),
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            self.timing.last_timed_frame_drain,
            frame_target_fps,
            animation_activity.needs_animation() || needs_text_caret_animation,
        );
        match cadence {
            TimedFrameCadence::Idle => event_loop.set_control_flow(ControlFlow::Wait),
            TimedFrameCadence::WaitUntil(next_frame) => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame));
            }
            TimedFrameCadence::DrainNow { next_wake } => {
                let expected_interval = animation_frame_interval(frame_target_fps);
                let elapsed_since_last = now.duration_since(self.timing.last_timed_frame_drain);
                let overdue = elapsed_since_last.saturating_sub(expected_interval);
                if overdue >= LATE_TIMED_FRAME_LOG_THRESHOLD
                    && elapsed_since_last <= LATE_TIMED_FRAME_MAX_CONTINUOUS_GAP
                {
                    warn!(
                        target: "radiant::debug::frame_profile",
                        event = "radiant.timed_frame.late",
                        target_fps = frame_target_fps,
                        elapsed_since_last_frame_us = elapsed_since_last.as_micros(),
                        expected_interval_us = expected_interval.as_micros(),
                        overdue_us = overdue.as_micros(),
                        animation_needs_frame_message = animation_activity.needs_frame_message(),
                        animation_needs_animation = animation_activity.needs_animation(),
                        needs_text_caret_animation,
                        redraw_requested = self.timing.redraw_requested,
                        redraw_pending_us = self
                            .timing
                            .redraw_requested_at
                            .map(|requested_at| now.duration_since(requested_at).as_micros())
                            .unwrap_or(0),
                        "Timed frame wakeup arrived late"
                    );
                }
                let outcome =
                    self.drain_timed_frame_now(now, animation_activity, needs_text_caret_animation);
                if outcome.exit_requested {
                    event_loop.exit();
                    return;
                }
                self.handle_route_outcome(event_loop, outcome);
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
            }
        }
    }
}
