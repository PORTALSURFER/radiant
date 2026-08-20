//! Winit application lifecycle for the generic native Vello runner.

use super::native_resource_maintenance::NATIVE_RESOURCE_MAINTENANCE_INTERVAL;
use super::{
    AuxiliaryWindowCloseAdmission, AuxiliaryWindowEventResult, CpuFrameObservationOwner,
    FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleKey, FrameScheduleLane,
    FrameScheduleRedrawEvidence, FrameWork, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    NativeGenericRunError, NativeInitializationStage, RuntimeUserEvent, TimedFrameCadence,
    animation_frame_interval, assess_cpu_frame_fairness, should_start_native_window_drag,
    should_toggle_native_window_maximized, slow_render_profile_enabled, timed_frame_cadence,
    timed_frame_target_fps,
};
use crate::runtime::{
    FrameProfile, NativeCpuFrameFairnessDiagnostics, NativeCpuFrameObservationDiagnostics,
    RuntimeAnimationActivity, RuntimeBridge,
};
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

fn is_native_interactive_window_event(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::Focused(_)
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::KeyboardInput { .. }
            | WindowEvent::ModifiersChanged(_)
    )
}

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
            #[cfg(target_os = "macos")]
            let semantic_proxy = event_proxy.clone();
            if let Err(error) = self.initialize_runtime(event_loop, event_proxy, &mut maintenance) {
                self.record_initialization_error_and_exit(event_loop, error);
            } else {
                #[cfg(target_os = "macos")]
                self.attach_native_semantic_accessibility(semantic_proxy);
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
        let native_interactive_arrival = (self.frame_diagnostics_enabled
            && is_native_interactive_window_event(&event))
        .then(Instant::now);
        if Some(window_id) != self.window.id {
            let Some(index) = self
                .auxiliary_windows
                .iter()
                .position(|window| window.window_id() == Some(window_id))
            else {
                return;
            };
            if let Some(arrived_at) = native_interactive_arrival {
                self.auxiliary_windows[index].record_native_interactive_arrival(arrived_at);
            }
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
            let auxiliary_owner = self.auxiliary_windows[index].effect_owner();
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
                message_origin,
                close_admission,
                terminal_cause,
                shutdown_requested,
                visual_deadline_completed: _,
            } = route_result;
            if let Some(Some(admission)) = admission {
                let capture = self.auxiliary_windows[index].take_cpu_frame_observation_capture();
                self.finish_cpu_frame_observation_with_capture(Some(admission), capture, false);
            }
            let close_was_attempted = close_admission.is_some();
            let accepted_close_messages = close_admission
                .and_then(|admission| self.accept_auxiliary_destructive_close(index, admission));
            let became_retiring = self.auxiliary_windows[index].is_retiring();
            if became_retiring {
                self.remove_cpu_frame_observation(&auxiliary_key);
                if !close_was_attempted {
                    self.core
                        .runtime
                        .retire_auxiliary_effect_owner(&auxiliary_owner);
                }
            }
            let frame_diagnostics =
                if shutdown_requested || terminal_cause.is_some() || became_retiring {
                    self.auxiliary_windows[index].discard_frame_diagnostics();
                    None
                } else {
                    self.auxiliary_windows[index].finalize_parent_frame_observation(false)
                };
            forward_auxiliary_frame_diagnostics(self, &auxiliary_key, frame_diagnostics);
            if shutdown_requested {
                self.admit_native_shutdown(event_loop, terminal_cause);
                return;
            }
            if let Some(error) = terminal_cause {
                self.record_auxiliary_terminal_cause_and_exit(event_loop, error);
                return;
            }
            if close_was_attempted && became_retiring {
                // Every accepted destructive close, including one with no
                // app-owned message or a post-terminal local fault, still
                // needs its bounded resource retirement opportunity. The
                // next independent sync may recreate a same-key projection
                // after this retirement-removal turn has completed.
                self.arm_retiring_auxiliary_maintenance_due_now();
            }
            if let Some(messages) = accepted_close_messages {
                if !messages.is_empty() {
                    self.dispatch_auxiliary_messages(event_loop, None, messages);
                }
            } else if !messages.is_empty() {
                self.dispatch_auxiliary_messages(event_loop, message_origin, messages);
            }
            return;
        }
        if let Some(arrived_at) = native_interactive_arrival {
            self.record_native_interactive_arrival(arrived_at);
        }
        match event {
            WindowEvent::CloseRequested if self.core.runtime.host_close_requested() => {
                self.admit_native_shutdown(event_loop, None);
            }
            WindowEvent::CloseRequested => {}
            WindowEvent::Resized(size) => {
                self.resize_surface(size);
                #[cfg(target_os = "macos")]
                self.invalidate_native_semantic_accessibility_geometry();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.update_native_dpi_scale(scale_factor);
                #[cfg(target_os = "macos")]
                self.invalidate_native_semantic_accessibility_geometry();
            }
            WindowEvent::Moved(_) => {
                self.observe_monitor_move();
                #[cfg(target_os = "macos")]
                self.invalidate_native_semantic_accessibility_geometry();
            }
            WindowEvent::ThemeChanged(theme) => self.observe_theme_change(Some(theme)),
            WindowEvent::Focused(false) => {
                self.window.native_window_focused = false;
                let routed = self.handle_focus_lost_before_external_drag();
                self.handle_route_outcome(event_loop, routed);
                if self.core.runtime.external_drag_armed() {
                    let outcome = self.launch_external_drag_if_armed();
                    self.handle_route_outcome(event_loop, outcome);
                }
                #[cfg(target_os = "macos")]
                self.republish_native_semantic_accessibility_passively();
            }
            WindowEvent::Focused(true) => {
                self.window.native_window_focused = true;
                let routed = self.handle_focus_regained_after_native_modal_loop();
                self.handle_route_outcome(event_loop, routed);
                #[cfg(target_os = "macos")]
                self.republish_native_semantic_accessibility_passively();
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
                    self.request_redraw_for_frame_work(FrameWork::None);
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
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let route = self.route_native_mouse_wheel_with_phase(delta, phase);
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
            WindowEvent::Ime(ime) => {
                let routed = self.route_native_ime_event(ime);
                self.handle_route_outcome(event_loop, routed);
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
            RuntimeUserEvent::ExternalDragCompleted {
                window_id,
                identity,
                result,
            } => self.handle_external_drag_completion(event_loop, window_id, identity, result),
            RuntimeUserEvent::NativeResourceMaintenanceRequested => {
                if self.is_closing() {
                    self.advance_native_closing(event_loop, Instant::now());
                } else if self.is_recovering() {
                    let _ = self.begin_native_resource_maintenance();
                } else if self.is_running() {
                    // Completion callbacks are wake-only. They may make the
                    // parent-owned retiring-child opportunity due, but never
                    // poll, sync, remove, or dispatch from this callback.
                    self.arm_retiring_auxiliary_maintenance_due_now();
                    self.wake_normal_native_resource_maintenance();
                    if let Some(generation) = self
                        .adapter
                        .as_ref()
                        .and_then(GenericNativeAdapterOwner::capture_generation)
                    {
                        for window in &mut self.auxiliary_windows {
                            window.wake_normal_native_resource_maintenance(generation);
                        }
                    }
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
                    self.invalidate_native_semantic_accessibility_geometry();
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeUserEvent::NativeSemanticAccessibilityQuery {
                window_id,
                generation,
                query,
            } => {
                if self.is_running()
                    && Some(window_id) == self.window.id
                    && self
                        .native_semantic_accessibility
                        .as_ref()
                        .is_some_and(|adapter| adapter.accepts_generation(generation))
                {
                    self.handle_native_semantic_accessibility_query(query);
                }
            }
            #[cfg(target_os = "macos")]
            RuntimeUserEvent::NativeNumericAccessibilityAction {
                window_id,
                generation,
                token,
                target,
                action,
            } => {
                if self.is_running()
                    && Some(window_id) == self.window.id
                    && self
                        .native_semantic_accessibility
                        .as_ref()
                        .is_some_and(|adapter| adapter.accepts_generation(generation))
                {
                    self.handle_native_numeric_accessibility_action(token, *target, action);
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
        let now = Instant::now();
        let retiring_auxiliary_maintenance_due = self.retiring_auxiliary_maintenance_is_due(now);
        if retiring_auxiliary_maintenance_due {
            // One shared turn covers the parent and all retiring children.
            // It intentionally excludes the primary and active auxiliary
            // maintenance-ticket paths for this opportunity.
            let mut maintenance = super::NativeResourceMaintenanceTurn::new();
            self.maintain_retiring_auxiliary_resources_with_turn(&mut maintenance);
            self.rearm_retiring_auxiliary_maintenance(now);
        }
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let primary_maintenance_deadline =
            self.normal_native_resource_maintenance_deadline(now, current_generation);
        let primary_window_ready = self.window.window.is_some();
        let primary_resources_ready = self.window.native_resources.is_some();
        if primary_window_ready && !primary_resources_ready {
            self.clear_native_visual_request_wake();
        }

        let mut demands = Vec::with_capacity(1 + self.auxiliary_windows.len());
        let primary_visual_schedule_eligible = primary_window_ready
            && primary_resources_ready
            && self.native_visual_request_schedule_is_eligible();
        if primary_visual_schedule_eligible || primary_maintenance_deadline.is_some() {
            let ordinary_schedule = primary_visual_schedule_eligible
                && self.native_visual_request_schedule_is_ordinary();
            self.observe_pending_window_activation();
            let animation_activity = self.core.animation_activity();
            let animation_activity = if ordinary_schedule {
                animation_activity
            } else {
                RuntimeAnimationActivity::idle()
            };
            let needs_text_caret_animation =
                ordinary_schedule && self.core.has_focused_text_input();
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
            demands.push(
                FrameScheduleDemand::from_cadence_with_requested_target_fps(
                    FrameScheduleKey::Primary,
                    cadence,
                    requested_target_fps,
                    frame_target_fps,
                    animation_activity,
                    needs_text_caret_animation,
                    FrameScheduleRedrawEvidence {
                        timed_repaint_deadline: ordinary_schedule
                            .then(|| self.core.timed_repaint_deadline())
                            .flatten(),
                        pending_redraw_requested: self.timing.redraw_requested,
                        pending_redraw_age: self.pending_redraw_age(now),
                        pending_redraw_retry_deadline: self.pending_redraw_retry_deadline(),
                        pending_redraw_fresh: self.timing.redraw_requested
                            && !self.pending_redraw_request_is_stale(now),
                    },
                )
                .with_maintenance_deadline(primary_maintenance_deadline),
            );
        }
        for window in &mut self.auxiliary_windows {
            if let Some(demand) = window.observe_frame_schedule(now, current_generation) {
                demands.push(demand);
            }
        }

        let maintenance_pending = demands
            .iter()
            .any(|demand| demand.maintenance_deadline().is_some());

        let plan = self.frame_scheduler.observe(
            now,
            &demands,
            FrameScheduleDeadlines {
                activation: self.activation_confirmation_deadline(now),
                maintenance: native_resource_maintenance_deadline(now, maintenance_pending),
                recovery: self.recovery_deadline(),
                ..FrameScheduleDeadlines::default()
            }
            .merge(FrameScheduleDeadlines {
                maintenance: self.retiring_auxiliary_maintenance_deadline(),
                ..FrameScheduleDeadlines::default()
            }),
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
            let selected_lane = plan.selected_lane.unwrap_or(FrameScheduleLane::Visual);
            match selected.clone() {
                FrameScheduleKey::Primary => {
                    let work = demand.work(now);
                    if selected_lane == FrameScheduleLane::Maintenance {
                        if !retiring_auxiliary_maintenance_due
                            && let Some(adapter_generation) = current_generation
                            && self.admit_native_resource_maintenance(
                                now,
                                &FrameScheduleKey::Primary,
                                adapter_generation,
                            )
                        {
                            self.record_frame_schedule_admission_with_lane(
                                selected,
                                selected_lane,
                                false,
                                false,
                            );
                        }
                    } else {
                        if let TimedFrameCadence::DrainNow { due_at, next_wake } = demand.cadence()
                        {
                            let _next_wake = next_wake;
                            if work.drain_timed_frame
                                && !self.should_defer_timed_frame_drain_for_pending_redraw(now)
                            {
                                let expected_interval =
                                    animation_frame_interval(demand.frame_target_fps());
                                let elapsed_since_last = now
                                    .saturating_duration_since(self.timing.last_timed_frame_drain);
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
                            self.record_frame_schedule_admission_with_lane(
                                selected,
                                selected_lane,
                                admission.visual_deadline_completed,
                                work.maintenance,
                            );
                            self.publish_staged_frame_diagnostics();
                        }
                    }
                }
                FrameScheduleKey::Auxiliary(key) => {
                    let work = demand.work(now);
                    if selected_lane == FrameScheduleLane::Maintenance {
                        if !retiring_auxiliary_maintenance_due {
                            let admitted = self.adapter.as_ref().is_some_and(|adapter| {
                                self.auxiliary_windows
                                    .iter_mut()
                                    .find(|window| window.key() == key)
                                    .is_some_and(|window| {
                                        window.admit_native_resource_maintenance(adapter, now)
                                    })
                            });
                            if admitted {
                                self.record_frame_schedule_admission_with_lane(
                                    selected,
                                    selected_lane,
                                    false,
                                    false,
                                );
                            }
                        }
                    } else {
                        let result = self.adapter.as_mut().and_then(|adapter| {
                            let mut observation =
                                self.cpu_frame_observation.as_mut().map(|ledger| {
                                    CpuFrameObservationOwner::new(ledger, selected.clone())
                                });
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
                                message_origin,
                                close_admission,
                                terminal_cause,
                                shutdown_requested,
                                visual_deadline_completed,
                            } = result;
                            debug_assert!(close_admission.is_none());
                            let frame_diagnostics =
                                if !shutdown_requested && terminal_cause.is_none() {
                                    self.record_frame_schedule_admission_with_lane(
                                        selected.clone(),
                                        selected_lane,
                                        visual_deadline_completed,
                                        work.maintenance,
                                    );
                                    if let Some(window) = self
                                        .auxiliary_windows
                                        .iter_mut()
                                        .find(|window| window.key() == key)
                                    {
                                        window.finalize_parent_frame_observation(true)
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
                            forward_auxiliary_frame_diagnostics(self, &selected, frame_diagnostics);
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
                                    event_loop,
                                    message_origin,
                                    messages,
                                );
                            }
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
    key: &FrameScheduleKey,
    handoff: Option<super::AuxiliaryFrameDiagnostics>,
) where
    Bridge: RuntimeBridge<Message>,
{
    if let Some(super::AuxiliaryFrameDiagnostics {
        mut diagnostics,
        profile_enabled,
    }) = handoff
    {
        diagnostics.cpu_fairness = runner
            .cpu_frame_fairness
            .as_ref()
            .map_or_else(NativeCpuFrameFairnessDiagnostics::default, |ledger| {
                ledger.project_frame_diagnostics(key)
            });
        diagnostics.cpu_observation = runner
            .cpu_frame_observation
            .as_ref()
            .map_or_else(NativeCpuFrameObservationDiagnostics::default, |ledger| {
                ledger.project_frame_diagnostics(key)
            });
        if runner.frame_diagnostics_enabled {
            runner
                .core
                .runtime
                .host_observe_frame_diagnostics(diagnostics);
        }
        if profile_enabled {
            runner
                .core
                .runtime
                .host_observe_frame_profile(FrameProfile::from(diagnostics));
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn accept_auxiliary_destructive_close(
        &mut self,
        index: usize,
        admission: AuxiliaryWindowCloseAdmission,
    ) -> Option<Vec<Message>> {
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let owner_is_active = self
            .core
            .runtime
            .auxiliary_effect_owner_is_active(&admission.owner);
        let window = self.auxiliary_windows.get(index)?;
        let owner_current =
            window.effect_owner().is_same_generation(&admission.owner) && owner_is_active;
        if !owner_current {
            if let Some(window) = self.auxiliary_windows.get_mut(index) {
                let _ = window.veto_native_lifecycle(admission.ticket);
            }
            return None;
        }
        let window = self.auxiliary_windows.get_mut(index)?;
        if !window.native_lifecycle_ticket_is_current(&admission.ticket, current_generation) {
            let _ = window.veto_native_lifecycle(admission.ticket);
            return None;
        }
        if !window.prepare_destructive_close(&admission.ticket) {
            let _ = window.veto_native_lifecycle(admission.ticket);
            return None;
        }
        if !self
            .core
            .runtime
            .retire_auxiliary_effect_owner(&admission.owner)
        {
            window.invalidate_terminal_convergence_stage_owner();
            // The child has already crossed its terminal boundary. Preserve
            // the committed app-owned close message and emit it exactly once
            // after local convergence, even when owner retirement reports a
            // post-terminal fault.
            let messages = window.take_close_message().into_iter().collect();
            self.arm_retiring_auxiliary_maintenance_due_now();
            return Some(messages);
        }
        if !window.complete_native_lifecycle(admission.ticket) {
            window.invalidate_terminal_convergence_stage_owner();
            // Completion faults are likewise post-terminal: converge only
            // this child, then retain the already-committed message's
            // exactly-once dispatch obligation.
            let messages = window.take_close_message().into_iter().collect();
            self.arm_retiring_auxiliary_maintenance_due_now();
            return Some(messages);
        }
        let messages = window.take_close_message().into_iter().collect();
        self.arm_retiring_auxiliary_maintenance_due_now();
        Some(messages)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn queue_accessibility_display_snapshot(
        &mut self,
        next: super::window_environment::AccessibilityDisplaySnapshot,
    ) {
        let previous = self.window.accessibility_display;
        self.window.accessibility_display = next;
        let environment = super::window_environment::environment_for_native_state(
            self.window.dpi_scale,
            self.window.environment.color_scheme(),
            next,
        );
        let changed = self.update_window_environment(environment);
        for change in super::window_environment::accessibility_display_changes(previous, next) {
            if changed {
                self.queue_window_environment_change(change);
            }
        }
    }

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
    use crate::gui_runtime::native_vello::generic_runtime::cpu_frame_observation::CpuFrameObservationCapture;
    use crate::gui_runtime::native_vello::generic_runtime::{
        CpuFramePendingRedrawAge, FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleKey,
        FrameScheduleRedrawEvidence, FrameWork, FrameWorkReason, animation_frame_interval,
        assess_cpu_frame_fairness, timed_frame_cadence, timed_frame_target_fps,
    };
    use crate::runtime::{
        AuxiliaryWindow, Command, FrameProfile, NativeCpuFrameFairnessDiagnostics,
        NativeCpuFrameFairnessDisposition, NativeCpuFrameObservationDiagnostics,
        NativeFrameDiagnostics, NativeWindowDiagnosticIdentity, ProfilingOptions,
        RuntimeAnimationActivity, RuntimeBridge, RuntimeFrameDiagnosticsHost,
        RuntimeFrameProfileHost, RuntimeHostCapabilities, UiSurface,
    };
    use crate::{
        application::empty, gui::types::Vector2, gui_runtime::NativeRunOptions, prelude::IntoView,
    };
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use winit::{
        dpi::{PhysicalPosition, PhysicalSize},
        event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum OrderedAuxiliaryEvent {
        Diagnostics {
            window_identity: Option<u64>,
            frame_sequence: Option<u64>,
            input_to_present_latency_us: Option<u64>,
            cpu_fairness: NativeCpuFrameFairnessDiagnostics,
            cpu_observation: NativeCpuFrameObservationDiagnostics,
        },
        Profile(Box<FrameProfile>),
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
            RuntimeHostCapabilities::new()
                .with_frame_diagnostics()
                .with_frame_profile()
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
                    input_to_present_latency_us: diagnostics.input_to_present_latency_us,
                    cpu_fairness: diagnostics.cpu_fairness,
                    cpu_observation: diagnostics.cpu_observation,
                });
        }
    }

    impl RuntimeFrameProfileHost for OrderedAuxiliaryBridge {
        fn observe_frame_profile(&mut self, profile: FrameProfile) {
            self.events
                .lock()
                .expect("ordering test event log should not be poisoned")
                .push(OrderedAuxiliaryEvent::Profile(Box::new(profile)));
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
            false,
        )
    }

    fn auxiliary_window_with_profile(
        parent_options: &crate::gui_runtime::NativeRunOptions,
        profiling: ProfilingOptions,
    ) -> AuxiliaryNativeWindow<u8> {
        let surface = crate::runtime::test_arc_surface(empty::<u8>().into_surface());
        let mut options = crate::gui_runtime::NativeRunOptions::default();
        options.frame.profiling = profiling;
        AuxiliaryNativeWindow::new(
            crate::runtime::AuxiliaryWindow::new("settings", options, surface),
            parent_options,
            Some(NativeWindowDiagnosticIdentity::from_runtime_value(2)),
            false,
            true,
        )
    }

    fn record_parent_observation(
        runner: &mut GenericNativeVelloRunner<OrderedAuxiliaryBridge, u8>,
        key: &FrameScheduleKey,
    ) {
        let ledger = runner
            .cpu_frame_observation
            .as_mut()
            .expect("enabled diagnostics should retain the parent observation ledger");
        let admission = ledger.begin(
            key.clone(),
            FrameWork::None,
            Some(60),
            CpuFramePendingRedrawAge::Unknown,
        );
        let mut capture = CpuFrameObservationCapture::default();
        capture.record_frame_work(FrameWork::PaintOnly {
            reason: FrameWorkReason::RoutedInput,
        });
        capture.mark_successful_presentation();
        ledger.finish(admission, capture, false);
    }

    fn auxiliary_profile_events(
        parent_profiling: ProfilingOptions,
        auxiliary_profiling: ProfilingOptions,
    ) -> Vec<OrderedAuxiliaryEvent> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut parent_options = crate::gui_runtime::NativeRunOptions::default();
        parent_options.frame.profiling = parent_profiling;
        let mut auxiliary = auxiliary_window_with_profile(&parent_options, auxiliary_profiling);
        let mut runner = GenericNativeVelloRunner::new(
            parent_options,
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
        let handoff = auxiliary.finalize_parent_frame_observation(false);
        forward_auxiliary_frame_diagnostics(
            &mut runner,
            &FrameScheduleKey::Auxiliary("settings".to_owned()),
            handoff,
        );

        events
            .lock()
            .expect("profile test event log should not be poisoned")
            .clone()
    }

    fn parent_with_destructive_auxiliary_close() -> (
        Box<GenericNativeVelloRunner<OrderedAuxiliaryBridge, u8>>,
        crate::runtime::AuxiliaryWindowOwner,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let options = NativeRunOptions::default();
        let mut parent = Box::new(GenericNativeVelloRunner::new(
            options.clone(),
            OrderedAuxiliaryBridge { events },
            Vector2::new(320.0, 240.0),
        ));
        let owner = parent
            .core
            .runtime
            .acquire_auxiliary_effect_owner("settings");
        let surface = crate::runtime::test_arc_surface(empty::<u8>().into_surface());
        parent
            .auxiliary_windows
            .push(AuxiliaryNativeWindow::new_with_owner(
                AuxiliaryWindow::new("settings", options, surface).on_close(7),
                &NativeRunOptions::default(),
                None,
                false,
                false,
                owner.clone(),
            ));
        (parent, owner)
    }

    #[test]
    fn destructive_auxiliary_close_preflights_owner_retires_before_message_and_completes() {
        let (mut parent, owner) = parent_with_destructive_auxiliary_close();
        let close = parent.auxiliary_windows[0].stage_destructive_close_for_test();
        let admission = close.close_admission.expect("close admission");

        let messages = parent
            .accept_auxiliary_destructive_close(0, admission)
            .expect("accepted close");
        assert_eq!(messages, [7]);
        assert!(parent.auxiliary_windows[0].is_retiring());
        assert!(parent.retiring_auxiliary_maintenance_is_due(Instant::now()));
        assert!(!owner.is_open());
        assert!(!parent.core.runtime.auxiliary_effect_owner_is_active(&owner));
        assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
    }

    #[test]
    fn destructive_auxiliary_close_owner_veto_is_inert_and_retains_message() {
        let (mut parent, owner) = parent_with_destructive_auxiliary_close();
        let close = parent.auxiliary_windows[0].stage_destructive_close_for_test();
        let admission = close.close_admission.expect("close admission");
        assert!(parent.core.runtime.retire_auxiliary_effect_owner(&owner));

        assert!(
            parent
                .accept_auxiliary_destructive_close(0, admission)
                .is_none()
        );
        assert!(parent.auxiliary_windows[0].is_admitted());
        assert!(!parent.auxiliary_windows[0].frame_stage_owner_has_in_flight());
        assert!(parent.auxiliary_windows[0].has_close_message_for_test());
    }

    #[test]
    fn destructive_auxiliary_close_currentness_veto_is_inert_and_retains_message() {
        let (mut parent, _owner) = parent_with_destructive_auxiliary_close();
        let close = parent.auxiliary_windows[0].stage_destructive_close_for_test();
        let admission = close.close_admission.expect("close admission");
        parent.auxiliary_windows[0].invalidate_terminal_convergence_stage_owner();

        assert!(
            parent
                .accept_auxiliary_destructive_close(0, admission)
                .is_none()
        );
        assert!(parent.is_running());
        assert!(parent.auxiliary_windows[0].is_admitted());
        assert!(!parent.auxiliary_windows[0].is_retiring());
        assert!(parent.auxiliary_windows[0].has_close_message_for_test());
    }

    #[test]
    fn destructive_auxiliary_close_does_not_mutate_sibling_or_parent_lifecycle() {
        let (mut parent, owner) = parent_with_destructive_auxiliary_close();
        let sibling_owner = parent
            .core
            .runtime
            .acquire_auxiliary_effect_owner("inspector");
        let sibling_surface = crate::runtime::test_arc_surface(empty::<u8>().into_surface());
        parent
            .auxiliary_windows
            .push(AuxiliaryNativeWindow::new_with_owner(
                AuxiliaryWindow::new("inspector", NativeRunOptions::default(), sibling_surface),
                &NativeRunOptions::default(),
                None,
                false,
                false,
                sibling_owner.clone(),
            ));

        let close = parent.auxiliary_windows[0].stage_destructive_close_for_test();
        let messages = parent
            .accept_auxiliary_destructive_close(0, close.close_admission.expect("close admission"))
            .expect("accepted close");

        assert_eq!(messages, [7]);
        assert!(parent.is_running());
        assert!(!owner.is_open());
        assert!(sibling_owner.is_open());
        assert!(parent.auxiliary_windows[1].is_admitted());
        assert!(!parent.auxiliary_windows[1].is_retiring());
    }

    #[test]
    fn auxiliary_profile_delivery_uses_auxiliary_profiling_option() {
        assert!(
            auxiliary_profile_events(ProfilingOptions::frame(), ProfilingOptions::off())
                .iter()
                .all(|event| !matches!(event, OrderedAuxiliaryEvent::Profile(_)))
        );

        let events = auxiliary_profile_events(ProfilingOptions::off(), ProfilingOptions::frame());
        assert!(matches!(
            events.first(),
            Some(OrderedAuxiliaryEvent::Diagnostics {
                frame_sequence: Some(41),
                ..
            })
        ));
        assert_eq!(
            events.get(1),
            Some(&OrderedAuxiliaryEvent::Profile(Box::new(FrameProfile {
                window_identity: Some(9),
                frame_sequence: Some(41),
                ..FrameProfile::from(NativeFrameDiagnostics::default())
            })))
        );
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
    fn native_interactive_event_scope_matches_routed_input_and_excludes_maintenance() {
        let device_id = DeviceId::dummy();
        let interactive_events = [
            WindowEvent::Focused(true),
            WindowEvent::CursorEntered { device_id },
            WindowEvent::CursorMoved {
                device_id,
                position: PhysicalPosition::new(4.0, 8.0),
            },
            WindowEvent::CursorLeft { device_id },
            WindowEvent::MouseInput {
                device_id,
                state: ElementState::Pressed,
                button: MouseButton::Left,
            },
            WindowEvent::MouseWheel {
                device_id,
                delta: MouseScrollDelta::LineDelta(0.0, 1.0),
                phase: TouchPhase::Moved,
            },
            WindowEvent::ModifiersChanged(Default::default()),
        ];
        assert!(
            interactive_events
                .iter()
                .all(super::is_native_interactive_window_event)
        );

        let non_interactive_events = [
            WindowEvent::CloseRequested,
            WindowEvent::Resized(PhysicalSize::new(320, 240)),
            WindowEvent::Moved(PhysicalPosition::new(10, 20)),
            WindowEvent::DroppedFile(std::path::PathBuf::from("sample.wav")),
            WindowEvent::HoveredFileCancelled,
            WindowEvent::RedrawRequested,
        ];
        assert!(
            non_interactive_events
                .iter()
                .all(|event| !super::is_native_interactive_window_event(event))
        );
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
            input_to_present_latency_us: Some(1234),
            ..NativeFrameDiagnostics::default()
        };

        auxiliary.stage_frame_diagnostics_for_test(diagnostics);
        assert_eq!(auxiliary.take_ready_frame_diagnostics(), None);
        let diagnostics = auxiliary.finalize_parent_frame_observation(false);
        let key = FrameScheduleKey::Auxiliary("settings".to_owned());
        forward_auxiliary_frame_diagnostics(&mut runner, &key, diagnostics);
        let _ = runner.core.runtime.dispatch_message(7);

        assert_eq!(
            *events
                .lock()
                .expect("ordering test event log should not be poisoned"),
            vec![
                OrderedAuxiliaryEvent::Diagnostics {
                    window_identity: Some(9),
                    frame_sequence: Some(41),
                    input_to_present_latency_us: Some(1234),
                    cpu_fairness: NativeCpuFrameFairnessDiagnostics::default(),
                    cpu_observation: NativeCpuFrameObservationDiagnostics::default(),
                },
                OrderedAuxiliaryEvent::Message(7),
            ]
        );
    }

    #[test]
    fn auxiliary_direct_diagnostics_project_the_keyed_parent_fairness_after_finalization() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut auxiliary = auxiliary_window_with_diagnostics();
        let mut runner = GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            OrderedAuxiliaryBridge {
                events: Arc::clone(&events),
            },
            crate::gui::types::Vector2::new(320.0, 40.0),
        );
        let key = FrameScheduleKey::Auxiliary("settings".to_owned());
        let now = Instant::now();
        let demand = FrameScheduleDemand::from_cadence_with_requested_target_fps(
            key.clone(),
            super::TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(7),
                next_wake: now + Duration::from_millis(16),
            },
            120,
            24,
            RuntimeAnimationActivity::paint_only_at(24),
            false,
            FrameScheduleRedrawEvidence::default(),
        );
        let demands = [demand];
        let plan = runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default());
        assert_eq!(plan.selected, Some(key.clone()));
        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);

        auxiliary.stage_frame_diagnostics_for_test(NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(9)),
            frame_sequence: Some(41),
            ..NativeFrameDiagnostics::default()
        });
        let diagnostics = auxiliary.finalize_parent_frame_observation(false);
        forward_auxiliary_frame_diagnostics(&mut runner, &key, diagnostics);

        assert_eq!(
            *events
                .lock()
                .expect("direct fairness event log should not be poisoned"),
            vec![OrderedAuxiliaryEvent::Diagnostics {
                window_identity: Some(9),
                frame_sequence: Some(41),
                input_to_present_latency_us: None,
                cpu_fairness: NativeCpuFrameFairnessDiagnostics {
                    available: true,
                    latest_disposition: NativeCpuFrameFairnessDisposition::Selected,
                    requested_target_fps: 120,
                    effective_target_fps: 24,
                    latest_due_lateness_us: Some(7_000),
                    selected_turns: 1,
                    ..NativeCpuFrameFairnessDiagnostics::default()
                },
                cpu_observation: NativeCpuFrameObservationDiagnostics::default(),
            }]
        );
    }

    #[test]
    fn auxiliary_scheduled_diagnostics_project_admission_after_parent_finalization() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut auxiliary = auxiliary_window_with_diagnostics();
        let mut runner = GenericNativeVelloRunner::new(
            crate::gui_runtime::NativeRunOptions::default(),
            OrderedAuxiliaryBridge {
                events: Arc::clone(&events),
            },
            crate::gui::types::Vector2::new(320.0, 40.0),
        );
        let key = FrameScheduleKey::Auxiliary("settings".to_owned());
        let now = Instant::now();
        let demand = FrameScheduleDemand::from_cadence_with_requested_target_fps(
            key.clone(),
            super::TimedFrameCadence::DrainNow {
                due_at: now - Duration::from_millis(7),
                next_wake: now + Duration::from_millis(16),
            },
            120,
            24,
            RuntimeAnimationActivity::paint_only_at(24),
            false,
            FrameScheduleRedrawEvidence::default(),
        );
        let demands = [demand];
        let plan = runner
            .frame_scheduler
            .observe(now, &demands, FrameScheduleDeadlines::default());
        assert_eq!(plan.selected, Some(key.clone()));
        assess_cpu_frame_fairness(now, &demands, None)
            .record_turn(runner.cpu_frame_fairness.as_mut().unwrap(), &plan);

        auxiliary.require_scheduled_frame_admission();
        auxiliary.stage_frame_diagnostics_for_test(NativeFrameDiagnostics {
            window_identity: Some(NativeWindowDiagnosticIdentity::from_runtime_value(9)),
            frame_sequence: Some(42),
            ..NativeFrameDiagnostics::default()
        });
        runner.record_frame_schedule_admission(key.clone());
        let diagnostics = auxiliary.finalize_parent_frame_observation(true);
        forward_auxiliary_frame_diagnostics(&mut runner, &key, diagnostics);
        let _ = runner.core.runtime.dispatch_message(7);

        assert_eq!(
            *events
                .lock()
                .expect("scheduled fairness event log should not be poisoned"),
            vec![
                OrderedAuxiliaryEvent::Diagnostics {
                    window_identity: Some(9),
                    frame_sequence: Some(42),
                    input_to_present_latency_us: None,
                    cpu_fairness: NativeCpuFrameFairnessDiagnostics {
                        available: true,
                        latest_disposition: NativeCpuFrameFairnessDisposition::Selected,
                        requested_target_fps: 120,
                        effective_target_fps: 24,
                        latest_due_lateness_us: Some(7_000),
                        selected_turns: 1,
                        cursor_admissions: 1,
                        latest_selected_was_admitted: true,
                        ..NativeCpuFrameFairnessDiagnostics::default()
                    },
                    cpu_observation: NativeCpuFrameObservationDiagnostics::default(),
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
        let no_present_diagnostics = auxiliary.finalize_parent_frame_observation(true);
        assert_eq!(no_present_diagnostics, None);
        let key = FrameScheduleKey::Auxiliary("settings".to_owned());
        forward_auxiliary_frame_diagnostics(&mut runner, &key, no_present_diagnostics);
        let _ = runner.core.runtime.dispatch_message(1);
        assert_eq!(
            *events
                .lock()
                .expect("ordering test event log should not be poisoned"),
            vec![OrderedAuxiliaryEvent::Message(1)]
        );

        record_parent_observation(&mut runner, &key);
        auxiliary.stage_frame_diagnostics_for_test(diagnostics);
        let frame_diagnostics = auxiliary.finalize_parent_frame_observation(false);
        forward_auxiliary_frame_diagnostics(&mut runner, &key, frame_diagnostics);
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
                    input_to_present_latency_us: None,
                    cpu_fairness: NativeCpuFrameFairnessDiagnostics::default(),
                    cpu_observation: NativeCpuFrameObservationDiagnostics {
                        available: true,
                        latest_outcome:
                            crate::runtime::NativeCpuFrameCompletionOutcome::SuccessfulPresentation,
                        latest_exact_interaction: true,
                        admitted_redraws: 1,
                        successful_presentations: 1,
                        ..NativeCpuFrameObservationDiagnostics::default()
                    },
                },
                OrderedAuxiliaryEvent::Message(2),
            ]
        );
    }
}
