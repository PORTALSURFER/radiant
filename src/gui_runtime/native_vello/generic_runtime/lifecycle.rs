//! Winit application lifecycle for the generic native Vello runner.

use super::{
    AuxiliaryWindowEventResult, CpuFrameObservationOwner, FrameScheduleDeadlines,
    FrameScheduleDemand, FrameScheduleKey, FrameScheduleRedrawEvidence, GenericNativeAdapterOwner,
    GenericNativeVelloRunner, NativeGenericRunError, NativeInitializationStage, RuntimeUserEvent,
    TimedFrameCadence, animation_frame_interval, assess_cpu_frame_fairness,
    should_start_native_window_drag, should_toggle_native_window_maximized,
    slow_render_profile_enabled, timed_frame_cadence, timed_frame_target_fps,
};
use crate::runtime::{NativeFrameDiagnostics, RuntimeBridge};
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
            if self.adapter.is_none() {
                self.record_initialization_error_and_exit(
                    event_loop,
                    NativeGenericRunError::NativeInitialization {
                        stage: NativeInitializationStage::DeviceAcquisition,
                        message: String::from("native adapter owner was not initialized"),
                    },
                );
                return;
            }
            let auxiliary_key =
                FrameScheduleKey::Auxiliary(self.auxiliary_windows[index].key().to_owned());
            let observe_redraw = matches!(&event, WindowEvent::RedrawRequested)
                && !self.auxiliary_windows[index].is_retiring();
            let admission = observe_redraw
                .then(|| self.begin_cpu_frame_observation(auxiliary_key.clone(), Instant::now()));
            let route_result = match self.adapter.as_mut() {
                Some(adapter) => {
                    let mut observation = self
                        .cpu_frame_observation
                        .as_mut()
                        .map(|ledger| CpuFrameObservationOwner::new(ledger, auxiliary_key.clone()));
                    self.auxiliary_windows[index].route_window_event(
                        event_loop,
                        event,
                        adapter,
                        observation.as_mut(),
                    )
                }
                None => {
                    if let Some(Some(admission)) = admission {
                        let capture =
                            self.auxiliary_windows[index].take_cpu_frame_observation_capture();
                        self.finish_cpu_frame_observation_with_capture(
                            Some(admission),
                            capture,
                            false,
                        );
                    }
                    return;
                }
            };
            let AuxiliaryWindowEventResult {
                messages,
                terminal_cause,
                shutdown_requested,
            } = route_result;
            if let Some(Some(admission)) = admission {
                let capture = self.auxiliary_windows[index].take_cpu_frame_observation_capture();
                self.finish_cpu_frame_observation_with_capture(Some(admission), capture, false);
            }
            let became_retiring = self.auxiliary_windows[index].is_retiring();
            if became_retiring {
                self.remove_cpu_frame_observation(&auxiliary_key);
            }
            let frame_diagnostics =
                if shutdown_requested || terminal_cause.is_some() || became_retiring {
                    self.auxiliary_windows[index].discard_frame_diagnostics();
                    None
                } else {
                    self.auxiliary_windows[index].finalize_parent_frame_diagnostics(false)
                };
            forward_auxiliary_frame_diagnostics(self, frame_diagnostics);
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
            WindowEvent::RedrawRequested => {
                self.redraw_and_exit_on_error(event_loop);
                self.publish_staged_frame_diagnostics();
            }
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
            let now = Instant::now();
            if self.recovery_expired(now) {
                self.admit_native_shutdown(event_loop, None);
            } else if let Some(deadline) = self.recovery_deadline() {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
            return;
        }
        if !self.is_running() {
            return;
        }
        let maintenance_pending = self
            .begin_native_resource_maintenance_and_wake_primary()
            .has_pending();
        let now = Instant::now();
        let primary_window_ready = self.window.window.is_some();
        let primary_resources_ready = self.window.native_resources.is_some();
        if primary_window_ready && !primary_resources_ready {
            self.timing.redraw_requested = false;
            self.timing.redraw_requested_at = None;
        }

        if primary_window_ready && primary_resources_ready {
            self.observe_pending_window_activation();
        }

        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let mut demands = Vec::with_capacity(1 + self.auxiliary_windows.len());
        if primary_window_ready && primary_resources_ready {
            let animation_activity = self.core.animation_activity();
            let needs_text_caret_animation = self.core.has_focused_text_input();
            let requested_target_fps = self.options.normalized_target_fps();
            let frame_target_fps = timed_frame_target_fps(
                requested_target_fps,
                animation_activity,
                needs_text_caret_animation,
            );
            let cadence = timed_frame_cadence(
                now,
                self.timing.last_timed_frame_drain,
                frame_target_fps,
                animation_activity.needs_animation() || needs_text_caret_animation,
            );
            demands.push(FrameScheduleDemand::from_cadence_with_requested_target_fps(
                FrameScheduleKey::Primary,
                cadence,
                requested_target_fps,
                frame_target_fps,
                animation_activity,
                needs_text_caret_animation,
                FrameScheduleRedrawEvidence {
                    timed_repaint_deadline: self.core.timed_repaint_deadline(),
                    pending_redraw_requested: self.timing.redraw_requested,
                    pending_redraw_age: self.pending_redraw_age(now),
                    pending_redraw_retry_deadline: self.pending_redraw_retry_deadline(),
                    pending_redraw_fresh: self.timing.redraw_requested
                        && !self.pending_redraw_request_is_stale(now),
                },
            ));
        }
        for window in &mut self.auxiliary_windows {
            if let Some(demand) = window.observe_frame_schedule(now, current_generation) {
                demands.push(demand);
            }
        }

        let plan = self.frame_scheduler.observe(
            now,
            &demands,
            FrameScheduleDeadlines {
                activation: self.activation_confirmation_deadline(now),
                maintenance: native_resource_maintenance_deadline(now, maintenance_pending),
                recovery: self.recovery_deadline(),
                ..FrameScheduleDeadlines::default()
            },
        );
        let shadow_fairness =
            assess_cpu_frame_fairness(now, &demands, self.cpu_frame_observation.as_ref());
        if let Some(ledger) = self.cpu_frame_fairness.as_mut() {
            shadow_fairness.record_turn(ledger, &plan);
        }

        if let Some(selected) = plan.selected.clone()
            && let Some(demand) = demands
                .iter()
                .find(|demand| demand.key() == &selected)
                .cloned()
        {
            match selected.clone() {
                FrameScheduleKey::Primary => {
                    let work = demand.work(now);
                    if let TimedFrameCadence::DrainNow { due_at, next_wake } = demand.cadence() {
                        let _next_wake = next_wake;
                        if work.drain_timed_frame
                            && !self.should_defer_timed_frame_drain_for_pending_redraw(now)
                        {
                            let expected_interval =
                                animation_frame_interval(demand.frame_target_fps());
                            let elapsed_since_last =
                                now.saturating_duration_since(self.timing.last_timed_frame_drain);
                            let overdue = now.saturating_duration_since(due_at);
                            if overdue >= LATE_TIMED_FRAME_LOG_THRESHOLD
                                && elapsed_since_last <= LATE_TIMED_FRAME_MAX_CONTINUOUS_GAP
                                && slow_render_profile_enabled()
                            {
                                warn!(
                                    target: "radiant::debug::frame_profile",
                                    event = "radiant.timed_frame.late",
                                    target_fps = demand.frame_target_fps(),
                                    elapsed_since_last_frame_us = elapsed_since_last.as_micros(),
                                    expected_interval_us = expected_interval.as_micros(),
                                    overdue_us = overdue.as_micros(),
                                    animation_needs_frame_message = demand
                                        .animation_activity()
                                        .needs_frame_message(),
                                    animation_needs_animation = demand
                                        .animation_activity()
                                        .needs_animation(),
                                    needs_text_caret_animation = demand
                                        .needs_text_caret_animation(),
                                    redraw_requested = self.timing.redraw_requested,
                                    redraw_pending_us = self
                                        .timing
                                        .redraw_requested_at
                                        .map(|requested_at| {
                                            now.duration_since(requested_at).as_micros()
                                        })
                                        .unwrap_or(0),
                                    "Timed frame wakeup arrived late"
                                );
                            }
                        }
                    }
                    let admission = self.admit_frame_schedule_work(now, &demand);
                    if admission.route_outcome {
                        if admission.outcome.exit_requested {
                            self.admit_native_shutdown(event_loop, None);
                            return;
                        }
                        self.handle_route_outcome_deferred_publication(
                            event_loop,
                            admission.outcome,
                        );
                    }
                    if admission.did_work {
                        self.record_frame_schedule_admission(selected);
                        self.publish_staged_frame_diagnostics();
                    }
                }
                FrameScheduleKey::Auxiliary(key) => {
                    let result = self.adapter.as_mut().and_then(|adapter| {
                        let mut observation = self
                            .cpu_frame_observation
                            .as_mut()
                            .map(|ledger| CpuFrameObservationOwner::new(ledger, selected.clone()));
                        self.auxiliary_windows
                            .iter_mut()
                            .find(|window| window.key() == key)
                            .and_then(|window| {
                                window.admit_frame_schedule_work(
                                    event_loop,
                                    adapter,
                                    observation.as_mut(),
                                    now,
                                    &demand,
                                )
                            })
                    });
                    if let Some(result) = result {
                        let super::AuxiliaryWindowEventResult {
                            messages,
                            terminal_cause,
                            shutdown_requested,
                        } = result;
                        let frame_diagnostics = if !shutdown_requested && terminal_cause.is_none() {
                            self.record_frame_schedule_admission(selected.clone());
                            if let Some(window) = self
                                .auxiliary_windows
                                .iter_mut()
                                .find(|window| window.key() == key)
                            {
                                window.finalize_parent_frame_diagnostics(true)
                            } else {
                                None
                            }
                        } else {
                            if let Some(window) = self
                                .auxiliary_windows
                                .iter_mut()
                                .find(|window| window.key() == key)
                            {
                                window.discard_frame_diagnostics();
                            }
                            None
                        };
                        forward_auxiliary_frame_diagnostics(self, frame_diagnostics);
                        if shutdown_requested {
                            self.admit_native_shutdown(event_loop, terminal_cause);
                            return;
                        }
                        if let Some(error) = terminal_cause {
                            self.record_auxiliary_terminal_cause_and_exit(event_loop, error);
                            return;
                        }
                        if !messages.is_empty() {
                            self.dispatch_auxiliary_messages_without_timed_frame(
                                event_loop, messages,
                            );
                        }
                    }
                }
            }
        }

        if let Some(deadline) = plan.deadlines.earliest() {
            let deadline = if primary_resources_ready {
                self.frame_wait_deadline(deadline)
            } else {
                deadline
            };
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
        if !primary_window_ready || !primary_resources_ready {
            self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
            return;
        }
        self.schedule_activation_confirmation_poll(event_loop, now);
        self.schedule_native_resource_maintenance(event_loop, now, maintenance_pending);
    }
}

fn native_resource_maintenance_deadline(now: Instant, pending: bool) -> Option<Instant> {
    pending.then(|| now + NATIVE_RESOURCE_MAINTENANCE_INTERVAL)
}

fn forward_auxiliary_frame_diagnostics<Bridge, Message>(
    runner: &mut GenericNativeVelloRunner<Bridge, Message>,
    diagnostics: Option<NativeFrameDiagnostics>,
) where
    Bridge: RuntimeBridge<Message>,
{
    if let Some(diagnostics) = diagnostics {
        runner
            .core
            .runtime
            .host_observe_frame_diagnostics(diagnostics);
    }
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
    use super::super::AuxiliaryNativeWindow;
    use super::{
        GenericNativeVelloRunner, NATIVE_RESOURCE_MAINTENANCE_INTERVAL,
        forward_auxiliary_frame_diagnostics, native_resource_maintenance_deadline,
    };
    use crate::gui_runtime::native_vello::generic_runtime::cpu_frame_fairness::{
        CpuFrameCadencePressure, CpuFrameCadenceRate,
    };
    use crate::gui_runtime::native_vello::generic_runtime::{
        FrameScheduleDemand, FrameScheduleKey, FrameScheduleRedrawEvidence,
        animation_frame_interval, assess_cpu_frame_fairness, timed_frame_cadence,
        timed_frame_target_fps,
    };
    use crate::runtime::{
        Command, NativeFrameDiagnostics, NativeWindowDiagnosticIdentity, RuntimeAnimationActivity,
        RuntimeBridge, RuntimeFrameDiagnosticsHost, RuntimeHostCapabilities, UiSurface,
    };
    use crate::{application::empty, prelude::IntoView};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    #[derive(Debug, PartialEq, Eq)]
    enum OrderedAuxiliaryEvent {
        Diagnostics {
            window_identity: Option<u64>,
            frame_sequence: Option<u64>,
        },
        Message(u8),
    }

    struct OrderedAuxiliaryBridge {
        events: Arc<Mutex<Vec<OrderedAuxiliaryEvent>>>,
    }

    impl RuntimeBridge<u8> for OrderedAuxiliaryBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<u8>> {
            crate::runtime::test_arc_surface(empty::<u8>().into_surface())
        }

        fn update(&mut self, message: u8) -> Command<u8> {
            self.events
                .lock()
                .expect("ordering test event log should not be poisoned")
                .push(OrderedAuxiliaryEvent::Message(message));
            Command::none()
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, u8> {
            RuntimeHostCapabilities::new().with_frame_diagnostics()
        }
    }

    impl RuntimeFrameDiagnosticsHost for OrderedAuxiliaryBridge {
        fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
            self.events
                .lock()
                .expect("ordering test event log should not be poisoned")
                .push(OrderedAuxiliaryEvent::Diagnostics {
                    window_identity: diagnostics
                        .window_identity
                        .map(NativeWindowDiagnosticIdentity::get),
                    frame_sequence: diagnostics.frame_sequence,
                });
        }
    }

    fn auxiliary_window_with_diagnostics() -> AuxiliaryNativeWindow<u8> {
        let surface = crate::runtime::test_arc_surface(empty::<u8>().into_surface());
        AuxiliaryNativeWindow::new(
            crate::runtime::AuxiliaryWindow::new(
                "settings",
                crate::gui_runtime::NativeRunOptions::default(),
                surface,
            ),
            &crate::gui_runtime::NativeRunOptions::default(),
            Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
            true,
        )
    }

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

    #[test]
    fn primary_activity_cap_keeps_requested_cadence_separate_from_effective_schedule() {
        let now = Instant::now();
        let last_timed_frame_drain = now - animation_frame_interval(24) - Duration::from_millis(3);
        let animation_activity = RuntimeAnimationActivity::paint_only_at(24);
        let needs_text_caret_animation = false;
        let requested_target_fps = 120;
        let frame_target_fps = timed_frame_target_fps(
            requested_target_fps,
            animation_activity,
            needs_text_caret_animation,
        );
        let cadence = timed_frame_cadence(
            now,
            last_timed_frame_drain,
            frame_target_fps,
            animation_activity.needs_animation() || needs_text_caret_animation,
        );
        let redraw = FrameScheduleRedrawEvidence::default();
        let primary = FrameScheduleDemand::from_cadence_with_requested_target_fps(
            FrameScheduleKey::Primary,
            cadence,
            requested_target_fps,
            frame_target_fps,
            animation_activity,
            needs_text_caret_animation,
            redraw,
        );
        let effective_only = FrameScheduleDemand::from_cadence(
            FrameScheduleKey::Primary,
            cadence,
            frame_target_fps,
            animation_activity,
            needs_text_caret_animation,
            redraw,
        );

        assert_eq!(frame_target_fps, 24);
        assert_eq!(primary.cadence(), effective_only.cadence());
        assert_eq!(primary.work(now), effective_only.work(now));
        assert_eq!(primary.deadlines(now), effective_only.deadlines(now));

        let demands = [primary];
        let evidence = assess_cpu_frame_fairness(now, &demands, None)
            .evidence_for(&FrameScheduleKey::Primary)
            .expect("primary demand should project fairness evidence");
        assert_eq!(evidence.requested_cadence, CpuFrameCadenceRate::Known(120));
        assert_eq!(evidence.effective_cadence, CpuFrameCadenceRate::Known(24));
        assert!(matches!(
            evidence.cadence,
            CpuFrameCadencePressure::Due { .. }
        ));
    }

    #[test]
    fn auxiliary_diagnostics_forward_before_messages_and_preserve_correlation() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut auxiliary = auxiliary_window_with_diagnostics();
        let mut runner = GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            OrderedAuxiliaryBridge {
                events: Arc::clone(&events),
            },
            crate::gui::types::Vector2::new(320.0, 40.0),
        );
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(9)),
            frame_sequence: Some(41),
            ..NativeFrameDiagnostics::default()
        };

        auxiliary.stage_frame_diagnostics_for_test(diagnostics);
        assert_eq!(auxiliary.take_ready_frame_diagnostics(), None);
        let diagnostics = auxiliary.finalize_parent_frame_diagnostics(false);
        forward_auxiliary_frame_diagnostics(&mut runner, diagnostics);
        let _ = runner.core.runtime.dispatch_message(7);

        assert_eq!(
            *events
                .lock()
                .expect("ordering test event log should not be poisoned"),
            vec![
                OrderedAuxiliaryEvent::Diagnostics {
                    window_identity: Some(9),
                    frame_sequence: Some(41),
                },
                OrderedAuxiliaryEvent::Message(7),
            ]
        );
    }

    #[test]
    fn auxiliary_scheduled_work_without_synchronous_present_publishes_nothing_until_later_redraw() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut auxiliary = auxiliary_window_with_diagnostics();
        let mut runner = GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            OrderedAuxiliaryBridge {
                events: Arc::clone(&events),
            },
            crate::gui::types::Vector2::new(320.0, 40.0),
        );
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(9)),
            frame_sequence: Some(42),
            ..NativeFrameDiagnostics::default()
        };

        auxiliary.require_scheduled_frame_admission();
        let no_present_diagnostics = auxiliary.finalize_parent_frame_diagnostics(true);
        assert_eq!(no_present_diagnostics, None);
        forward_auxiliary_frame_diagnostics(&mut runner, no_present_diagnostics);
        let _ = runner.core.runtime.dispatch_message(1);
        assert_eq!(
            *events
                .lock()
                .expect("ordering test event log should not be poisoned"),
            vec![OrderedAuxiliaryEvent::Message(1)]
        );

        auxiliary.stage_frame_diagnostics_for_test(diagnostics);
        let frame_diagnostics = auxiliary.finalize_parent_frame_diagnostics(false);
        forward_auxiliary_frame_diagnostics(&mut runner, frame_diagnostics);
        let _ = runner.core.runtime.dispatch_message(2);
        assert_eq!(
            *events
                .lock()
                .expect("ordering test event log should not be poisoned"),
            vec![
                OrderedAuxiliaryEvent::Message(1),
                OrderedAuxiliaryEvent::Diagnostics {
                    window_identity: Some(9),
                    frame_sequence: Some(42),
                },
                OrderedAuxiliaryEvent::Message(2),
            ]
        );
    }
}
