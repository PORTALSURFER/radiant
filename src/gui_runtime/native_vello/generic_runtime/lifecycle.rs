//! Winit application lifecycle for the generic native Vello runner.

use super::{
    AuxiliaryWindowEventResult, GenericNativeVelloRunner, NativeGenericRunError,
    NativeInitializationStage, RuntimeUserEvent, TimedFrameCadence, animation_frame_interval,
    should_start_native_window_drag, should_toggle_native_window_maximized,
    slow_render_profile_enabled, timed_frame_cadence, timed_frame_target_fps,
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
const NATIVE_RESOURCE_MAINTENANCE_INTERVAL: Duration = Duration::from_millis(16);

impl<Bridge, Message> ApplicationHandler<RuntimeUserEvent>
    for GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.is_running() {
            return;
        }
        if self.should_initialize_runtime() {
            let mut maintenance = self.begin_native_resource_maintenance();
            self.install_application_reopen_handler_if_needed();
            let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy() else {
                self.record_initialization_error_and_exit(
                    event_loop,
                    NativeGenericRunError::NativeInitialization {
                        stage: NativeInitializationStage::DeviceAcquisition,
                        message: String::from("native event-loop proxy was not installed"),
                    },
                );
                return;
            };
            if let Err(error) = self.initialize_runtime(event_loop, event_proxy, &mut maintenance) {
                self.record_initialization_error_and_exit(event_loop, error);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !self.is_running() {
            return;
        }
        if Some(window_id) != self.window.id {
            let Some(index) = self
                .auxiliary_windows
                .iter()
                .position(|window| window.window_id() == Some(window_id))
            else {
                return;
            };
            let Some(adapter) = self.adapter.as_mut() else {
                self.record_initialization_error_and_exit(
                    event_loop,
                    NativeGenericRunError::NativeInitialization {
                        stage: NativeInitializationStage::DeviceAcquisition,
                        message: String::from("native adapter owner was not initialized"),
                    },
                );
                return;
            };
            let AuxiliaryWindowEventResult {
                messages,
                terminal_cause,
                shutdown_requested,
            } = self.auxiliary_windows[index].route_window_event(event_loop, event, adapter);
            if shutdown_requested {
                self.admit_native_shutdown(event_loop, terminal_cause);
                return;
            }
            if let Some(error) = terminal_cause {
                self.record_auxiliary_terminal_cause_and_exit(event_loop, error);
                return;
            }
            if !messages.is_empty() {
                self.dispatch_auxiliary_messages(event_loop, messages);
            }
            return;
        }
        match event {
            WindowEvent::CloseRequested if self.core.runtime.host_close_requested() => {
                self.admit_native_shutdown(event_loop, None);
            }
            WindowEvent::CloseRequested => {}
            WindowEvent::Resized(size) => self.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::Moved(_) => self.observe_monitor_move(),
            WindowEvent::ThemeChanged(theme) => self.observe_theme_change(Some(theme)),
            WindowEvent::Focused(false) => {
                let routed = self.handle_focus_lost_before_external_drag();
                self.handle_route_outcome(event_loop, routed);
                if self.core.runtime.external_drag_armed() {
                    let outcome = self.launch_external_drag_if_armed();
                    self.handle_route_outcome(event_loop, outcome);
                }
            }
            WindowEvent::Focused(true) => {
                let routed = self.handle_focus_regained_after_native_modal_loop();
                self.handle_route_outcome(event_loop, routed);
            }
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
                    && should_toggle_native_window_maximized(
                        &self.options,
                        position,
                        button,
                        route.outcome.routed,
                        route.double_click,
                    )
                    && let Some(window) = self.window.window.clone()
                {
                    window.set_maximized(!window.is_maximized());
                    // Native zoom transitions can resize outside a live-resize
                    // gesture. Force one complete app-owned scene refresh so
                    // retained and composited layers cannot remain at the old
                    // viewport while the new surface is already visible.
                    self.defer_interactive_scene_rebuild();
                    window.request_redraw();
                } else if route.is_pressed()
                    && let (Some(position), Some(button)) = (route.position, route.button)
                    && should_start_native_window_drag(
                        &self.options,
                        position,
                        button,
                        route.outcome.routed,
                    )
                    && let Some(window) = self.window.window.as_ref()
                    && let Err(err) = super::window::drag_app_owned_window(window, &self.options)
                {
                    warn!("radiant generic native vello: app-owned window drag failed: {err}");
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
            WindowEvent::RedrawRequested => self.redraw_and_exit_on_error(event_loop),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RuntimeUserEvent) {
        match event {
            RuntimeUserEvent::RepaintRequested => {
                if !self.is_running() {
                    self.runtime_wakeup.clear_pending();
                    return;
                }
                self.runtime_wakeup.clear_pending();
                let outcome = self.core.drain_runtime_messages();
                self.handle_route_outcome(event_loop, outcome);
            }
            RuntimeUserEvent::OpenFiles(paths) => {
                if self.is_running() {
                    self.handle_native_file_open(event_loop, paths);
                }
            }
            RuntimeUserEvent::ApplicationReopenRequested => {
                if self.is_running() {
                    self.handle_application_reopen_intent();
                    self.observe_pending_window_activation();
                }
            }
            RuntimeUserEvent::DeviceLost {
                registration,
                generation,
                message,
            } => self.handle_device_lost_event(event_loop, generation, registration, message),
            RuntimeUserEvent::DeviceRecoveryReady { episode } => {
                self.handle_device_recovery_ready(event_loop, episode)
            }
            RuntimeUserEvent::RenderDeviceError {
                registration,
                generation,
                kind,
                message,
            } => self.handle_render_device_error_event(
                event_loop,
                generation,
                registration,
                kind,
                message,
            ),
            RuntimeUserEvent::NativeResourceMaintenanceRequested => {
                if self.is_closing() {
                    self.advance_native_closing(event_loop, Instant::now());
                } else if self.is_recovering() {
                    let _ = self.begin_native_resource_maintenance();
                } else if self.is_running() {
                    let _ = self.begin_native_resource_maintenance_and_wake_primary();
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeUserEvent::AccessibilityDisplayChanged => {
                if self.is_running() {
                    let snapshot = super::accessibility::current_snapshot();
                    self.queue_accessibility_display_snapshot(snapshot);
                    for window in &mut self.auxiliary_windows {
                        window.queue_accessibility_display_snapshot(snapshot);
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.is_closing() {
            self.advance_native_closing(event_loop, Instant::now());
            return;
        }
        if self.is_recovering() {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }
        if !self.is_running() {
            return;
        }
        let maintenance_pending = self
            .begin_native_resource_maintenance_and_wake_primary()
            .has_pending();
        let now = Instant::now();
        if self.window.window.is_none() {
            event_loop.set_control_flow(ControlFlow::Wait);
            self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
            return;
        }
        if self.window.native_resources.is_none() {
            self.timing.redraw_requested = false;
            self.timing.redraw_requested_at = None;
            event_loop.set_control_flow(ControlFlow::Wait);
            self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
            return;
        }
        self.observe_pending_window_activation();
        let animation_activity = self.core.animation_activity();
        if self.core.advance_timed_repaints(now) {
            self.rebuild_scene();
            self.request_redraw_for_frame_work(super::FrameWork::RebuildScene {
                reason: super::FrameWorkReason::RuntimeSurfaceRepaint,
                mode: super::SceneRebuildMode::Immediate,
            });
        }
        let timed_repaint_deadline = self.core.timed_repaint_deadline();
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
            TimedFrameCadence::Idle => match timed_repaint_deadline {
                Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
                None => event_loop.set_control_flow(ControlFlow::Wait),
            },
            TimedFrameCadence::WaitUntil(next_frame) => {
                let next_wake =
                    timed_repaint_deadline.map_or(next_frame, |deadline| next_frame.min(deadline));
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(self.frame_wait_deadline(next_wake)));
            }
            TimedFrameCadence::DrainNow { next_wake } => {
                if self.should_defer_timed_frame_drain_for_pending_redraw(now) {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        self.frame_wait_deadline(next_wake),
                    ));
                    self.schedule_activation_confirmation_poll(event_loop, now);
                    self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
                    return;
                }
                let expected_interval = animation_frame_interval(frame_target_fps);
                let elapsed_since_last = now.duration_since(self.timing.last_timed_frame_drain);
                let overdue = elapsed_since_last.saturating_sub(expected_interval);
                if overdue >= LATE_TIMED_FRAME_LOG_THRESHOLD
                    && elapsed_since_last <= LATE_TIMED_FRAME_MAX_CONTINUOUS_GAP
                    && slow_render_profile_enabled()
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
                    self.admit_native_shutdown(event_loop, None);
                    return;
                }
                self.handle_route_outcome(event_loop, outcome);
                let next_wake =
                    timed_repaint_deadline.map_or(next_wake, |deadline| next_wake.min(deadline));
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_wake));
            }
        }
        self.schedule_activation_confirmation_poll(event_loop, now);
        self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
    }
}

fn native_resource_maintenance_deadline(now: Instant, pending: bool) -> Option<Instant> {
    pending.then(|| now + NATIVE_RESOURCE_MAINTENANCE_INTERVAL)
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn schedule_native_resource_maintenance(
        &self,
        event_loop: &ActiveEventLoop,
        now: Instant,
        pending: bool,
    ) {
        let Some(deadline) = native_resource_maintenance_deadline(now, pending) else {
            return;
        };
        match event_loop.control_flow() {
            ControlFlow::Poll => {}
            ControlFlow::Wait => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            ControlFlow::WaitUntil(current) if deadline < current => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            ControlFlow::WaitUntil(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NATIVE_RESOURCE_MAINTENANCE_INTERVAL, native_resource_maintenance_deadline};
    use std::time::{Duration, Instant};

    #[test]
    fn pending_maintenance_gets_a_bounded_future_opportunity() {
        let now = Instant::now();
        let deadline = native_resource_maintenance_deadline(now, true)
            .expect("pending maintenance should schedule a future opportunity");

        assert!(deadline > now);
        assert!(deadline.duration_since(now) <= NATIVE_RESOURCE_MAINTENANCE_INTERVAL);
    }

    #[test]
    fn idle_maintenance_does_not_create_a_busy_loop_deadline() {
        let now = Instant::now();

        assert_eq!(native_resource_maintenance_deadline(now, false), None);
        assert!(NATIVE_RESOURCE_MAINTENANCE_INTERVAL >= Duration::from_millis(1));
    }
}
