//! Winit application lifecycle for the generic native Vello runner.

use super::auxiliary::AuxiliaryNativeWindow;
use super::frame_scheduler_policy::NativeInputStageDisposition;
use super::lifecycle_pointer::finalize_native_immediate_transient_route;
use super::native_discrete_input_stage::NativeDiscreteInputKind;
use super::native_immediate_transient_stage::NativeImmediateTransientKind;
use super::native_pointer_ingress::{
    GestureInput, NativeGestureSample, NativeTouchSample, normalize_gesture, normalize_touch,
};
use super::native_resource_maintenance::NATIVE_RESOURCE_MAINTENANCE_INTERVAL;
use super::runner::select_due_admitted_auxiliary_index;
use super::{
    AuxiliaryWindowCloseAdmission, AuxiliaryWindowEventResult, CpuFrameObservationOwner,
    FrameScheduleDeadlines, FrameScheduleDemand, FrameScheduleKey, FrameScheduleLane,
    FrameScheduleRedrawEvidence, FrameWork, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    GenericRouteOutcome, NativeGenericRunError, NativeGpuTimingRoute, NativeInitializationStage,
    NativeLifecycle, RuntimeUserEvent, TimedFrameCadence, animation_frame_interval,
    assess_cpu_frame_fairness, should_start_native_window_drag,
    should_toggle_native_window_maximized, slow_render_profile_enabled, timed_frame_cadence,
    timed_frame_target_fps,
};
use crate::gui::input::InputTimestamp;
use crate::gui::pointer_ingress::{
    DeviceKind, GestureIngress, GestureIngressDisposition, PointerButtons, PointerIngress,
    PointerIngressDisposition, PointerPhase,
};
use crate::runtime::{
    FrameGpuTimingSample, FrameProfile, NativeCpuFrameFairnessDiagnostics,
    NativeCpuFrameObservationDiagnostics, RuntimeAnimationActivity, RuntimeBridge,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeGpuTimingReadyHandling {
    Deliver,
    Discard,
}

const fn native_gpu_timing_ready_handling(
    lifecycle: NativeLifecycle,
) -> NativeGpuTimingReadyHandling {
    if lifecycle.is_running() {
        NativeGpuTimingReadyHandling::Deliver
    } else {
        NativeGpuTimingReadyHandling::Discard
    }
}

fn should_request_native_maximize_redraw(outcome: GenericRouteOutcome) -> bool {
    !matches!(
        outcome.native_input_stage_disposition(),
        Some(NativeInputStageDisposition::DeferLowerPriority)
    )
}

pub(super) struct NativeGestureRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) deferred_wheel_effects: super::gpu_surface_wheel::DeferredWheelRouteEffects,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn retain_native_mouse_device(
        &mut self,
        device_id: winit::event::DeviceId,
        hover: Option<bool>,
    ) {
        self.input.last_native_mouse_device = Some(device_id);
        if let Some(hover) = hover {
            let _ = self
                .input
                .native_pointer_ingress
                .set_hover(device_id, hover);
        } else {
            let _ = self
                .input
                .native_pointer_ingress
                .retain_device(device_id, DeviceKind::Mouse);
        }
    }

    pub(super) fn normalize_native_touch_transient(
        &mut self,
        event_loop: &ActiveEventLoop,
        touch: winit::event::Touch,
    ) {
        let timestamp = InputTimestamp::capture();
        let Some(adapter_generation) = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
        else {
            return;
        };
        let Some(ticket) = self.begin_native_immediate_transient_event(
            event_loop,
            NativeImmediateTransientKind::Touch(touch.phase),
            timestamp,
            adapter_generation,
            true,
        ) else {
            return;
        };
        let Some(ticket) =
            self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
        else {
            return;
        };
        let modifiers = self.pointer_modifiers();
        if let Ok(sample) = normalize_touch(
            &mut self.input.native_pointer_ingress,
            touch,
            self.window.dpi_scale,
            modifiers,
            timestamp,
        ) {
            let _ = self.dispatch_native_touch_sample(sample);
        }
        let _ = self.complete_native_immediate_transient(ticket);
    }

    fn dispatch_native_touch_sample(
        &mut self,
        sample: NativeTouchSample,
    ) -> PointerIngressDisposition {
        let phase = match sample.phase {
            winit::event::TouchPhase::Started => PointerPhase::Started {
                button: crate::widgets::PointerButton::Primary,
            },
            winit::event::TouchPhase::Moved => PointerPhase::Moved,
            winit::event::TouchPhase::Ended => PointerPhase::Ended {
                button: crate::widgets::PointerButton::Primary,
            },
            winit::event::TouchPhase::Cancelled => PointerPhase::Cancelled,
        };
        let sequence_range = self.input.input_sequence_allocator.allocate();
        if matches!(sample.phase, winit::event::TouchPhase::Started) {
            PointerIngress::new(
                DeviceKind::Touch,
                sample.device,
                sample.contact,
                phase,
                sample.position,
                PointerButtons::empty(),
                sample.modifiers,
                sample.pressure,
                sample.tilt,
                Some(sample.timestamp),
                sequence_range,
            )
            .map(|ingress| {
                let admission = self
                    .core
                    .runtime
                    .dispatch_pointer_ingress_with_admission(ingress);
                if let Some(token) = admission.sequence_token() {
                    let _ = self.input.native_pointer_ingress.set_token_for_identity(
                        sample.device,
                        sample.contact,
                        token,
                    );
                }
                admission.disposition()
            })
            .unwrap_or(PointerIngressDisposition::Invalid)
        } else {
            sample
                .sequence_token
                .map_or(PointerIngressDisposition::Stale, |token| {
                    self.core.runtime.dispatch_native_pointer_continuation(
                        DeviceKind::Touch,
                        sample.device,
                        sample.contact,
                        token,
                        phase,
                        sample.position,
                        PointerButtons::empty(),
                        sample.modifiers,
                        sample.pressure,
                        sample.tilt,
                        Some(sample.timestamp),
                        sequence_range,
                    )
                })
        }
    }

    pub(super) fn normalize_native_gesture_transient(
        &mut self,
        event_loop: &ActiveEventLoop,
        kind: NativeImmediateTransientKind,
        device_id: winit::event::DeviceId,
        gesture: GestureInput,
    ) {
        let timestamp = InputTimestamp::capture();
        let Some(adapter_generation) = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation)
        else {
            return;
        };
        let Some(ticket) = self.begin_native_immediate_transient_event(
            event_loop,
            kind,
            timestamp,
            adapter_generation,
            true,
        ) else {
            return;
        };
        let Some(ticket) =
            self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
        else {
            return;
        };
        let modifiers = self.pointer_modifiers();
        self.input
            .native_pointer_ingress
            .retain_gesture_device(self.core.runtime.retained_gesture_device());
        let normalized = normalize_gesture(
            &mut self.input.native_pointer_ingress,
            device_id,
            gesture,
            self.window.dpi_scale,
            modifiers,
            timestamp,
        );
        let route = if let Ok(Ok(sample)) = normalized {
            Some(self.route_native_gesture_sample(sample))
        } else {
            // Unsupported desktop pan and malformed transports complete their
            // ticket once without fabricating a routed gesture.
            None
        };
        let outcome = route
            .as_ref()
            .map_or(GenericRouteOutcome::default(), |route| route.outcome);
        if let Some(outcome) = self.complete_native_immediate_transient_route(ticket, outcome) {
            if let Some(route) = route {
                self.apply_deferred_wheel_route_effects(
                    route.deferred_wheel_effects,
                    outcome.native_input_stage_disposition(),
                );
            }
            self.handle_route_outcome(event_loop, outcome);
        }
    }

    pub(super) fn route_native_gesture_sample(
        &mut self,
        sample: NativeGestureSample,
    ) -> NativeGestureRoute {
        let phase = match sample.phase {
            winit::event::TouchPhase::Started => crate::gui::pointer_ingress::GesturePhase::Started,
            winit::event::TouchPhase::Moved => crate::gui::pointer_ingress::GesturePhase::Changed,
            winit::event::TouchPhase::Ended => crate::gui::pointer_ingress::GesturePhase::Ended,
            winit::event::TouchPhase::Cancelled => {
                crate::gui::pointer_ingress::GesturePhase::Cancelled
            }
        };
        let Ok(gesture) = GestureIngress::new(
            sample.kind,
            phase,
            sample.unit,
            crate::gui::types::Vector2::new(sample.value, 0.0),
            sample.device,
            None,
            sample.modifiers,
            Some(sample.timestamp),
            self.input.input_sequence_allocator.allocate(),
        ) else {
            self.core
                .runtime
                .reject_native_gesture_continuation(sample.device, sample.kind, phase);
            self.input
                .native_pointer_ingress
                .retain_gesture_device(self.core.runtime.retained_gesture_device());
            return NativeGestureRoute {
                outcome: self.core.route_outcome(false),
                deferred_wheel_effects: Default::default(),
            };
        };
        // Earlier coalesced wheel samples route before this boundary; visual
        // work remains deferred until the same native ticket completes.
        let deferred_wheel_effects = self.route_pending_wheel_input_for_immediate_transient();
        let disposition = self.core.runtime.dispatch_native_gesture_ingress(gesture);
        self.input
            .native_pointer_ingress
            .retain_gesture_device(self.core.runtime.retained_gesture_device());
        let outcome = self.core.route_outcome(matches!(
            disposition,
            GestureIngressDisposition::RoutedWidget(_)
                | GestureIngressDisposition::RoutedContainer(_)
        ));
        NativeGestureRoute {
            outcome,
            deferred_wheel_effects,
        }
    }
}

fn is_native_interactive_window_event(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::Focused(_)
            | WindowEvent::CursorEntered { .. }
            | WindowEvent::CursorMoved { .. }
            | WindowEvent::CursorLeft { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Touch(_)
            | WindowEvent::PinchGesture { .. }
            | WindowEvent::PanGesture { .. }
            | WindowEvent::RotationGesture { .. }
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
            if Some(window_id) == self.window.id && matches!(&event, WindowEvent::Focused(true)) {
                self.record_normal_window_activation_intent("focus-regained-during-lifecycle");
            }
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
                native_discrete_input_route: pending_native_discrete_input_route,
                native_immediate_transient_route: pending_native_immediate_transient_route,
            } = route_result;
            let mut pending_native_discrete_input_route = pending_native_discrete_input_route;
            let mut pending_native_immediate_transient_route =
                pending_native_immediate_transient_route;
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
            let frame_gpu_timing =
                if shutdown_requested || terminal_cause.is_some() || became_retiring {
                    self.auxiliary_windows[index].discard_frame_gpu_timing();
                    None
                } else {
                    self.auxiliary_windows[index].take_ready_frame_gpu_timing()
                };
            forward_auxiliary_frame_diagnostics(self, &auxiliary_key, frame_diagnostics);
            forward_auxiliary_frame_gpu_timing(self, frame_gpu_timing);
            if shutdown_requested {
                self.cancel_auxiliary_native_discrete_input_route(
                    index,
                    pending_native_discrete_input_route.take(),
                );
                self.cancel_auxiliary_native_immediate_transient_route(
                    index,
                    pending_native_immediate_transient_route.take(),
                );
                self.admit_native_shutdown(event_loop, terminal_cause);
                return;
            }
            if let Some(error) = terminal_cause {
                self.cancel_auxiliary_native_discrete_input_route(
                    index,
                    pending_native_discrete_input_route.take(),
                );
                self.cancel_auxiliary_native_immediate_transient_route(
                    index,
                    pending_native_immediate_transient_route.take(),
                );
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
            if became_retiring {
                self.cancel_auxiliary_native_discrete_input_route(
                    index,
                    pending_native_discrete_input_route.take(),
                );
                self.cancel_auxiliary_native_immediate_transient_route(
                    index,
                    pending_native_immediate_transient_route.take(),
                );
            }
            if let Some(messages) = accepted_close_messages {
                self.cancel_auxiliary_native_discrete_input_route(
                    index,
                    pending_native_discrete_input_route.take(),
                );
                self.cancel_auxiliary_native_immediate_transient_route(
                    index,
                    pending_native_immediate_transient_route.take(),
                );
                if !messages.is_empty() {
                    self.dispatch_auxiliary_messages(event_loop, None, messages, None, None);
                }
            } else if !messages.is_empty()
                || pending_native_discrete_input_route.is_some()
                || pending_native_immediate_transient_route.is_some()
            {
                self.dispatch_auxiliary_messages(
                    event_loop,
                    message_origin,
                    messages,
                    pending_native_discrete_input_route.map(|route| (index, route)),
                    pending_native_immediate_transient_route.map(|route| (index, route)),
                );
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
            WindowEvent::Occluded(occluded) => self.handle_surface_occlusion(occluded),
            WindowEvent::Focused(false) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::Focused(false),
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                self.window.native_window_focused = false;
                let routed = self.handle_focus_lost_before_external_drag();
                let launch_external_drag = self.core.runtime.external_drag_armed();
                if let Some(routed) = finalize_native_immediate_transient_route(
                    self.complete_native_immediate_transient(ticket),
                    routed,
                    launch_external_drag,
                    || self.launch_external_drag_if_armed(),
                ) {
                    self.handle_route_outcome(event_loop, routed);
                    #[cfg(target_os = "macos")]
                    self.republish_native_semantic_accessibility_passively();
                }
            }
            WindowEvent::Focused(true) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::Focused(true),
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                self.window.native_window_focused = true;
                let routed = self.handle_focus_regained_after_native_modal_loop();
                if let Some(routed) = self.complete_native_immediate_transient_route(ticket, routed)
                {
                    self.apply_pending_normal_window_activation("focus-regained");
                    self.handle_route_outcome(event_loop, routed);
                    #[cfg(target_os = "macos")]
                    self.republish_native_semantic_accessibility_passively();
                }
            }
            WindowEvent::CursorEntered { device_id } => {
                self.retain_native_mouse_device(device_id, Some(true));
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorEntered,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                self.handle_cursor_entered();
                if let Some(routed) = self.complete_native_immediate_transient_route(
                    ticket,
                    GenericRouteOutcome::default(),
                ) {
                    self.handle_route_outcome(event_loop, routed);
                }
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                self.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorMoved,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                let mut route = self.route_cursor_moved_with_timestamp(position, timestamp);
                if let Some(outcome) =
                    self.complete_native_immediate_transient_route(ticket, route.outcome)
                {
                    route.outcome = outcome;
                    self.apply_cursor_moved_route(route);
                }
            }
            WindowEvent::HoveredFile(path) => self.handle_native_file_hover(event_loop, path),
            WindowEvent::HoveredFileCancelled => self.handle_native_file_cancel(event_loop),
            WindowEvent::DroppedFile(path) => self.handle_native_file_drop(event_loop, path),
            WindowEvent::CursorLeft { device_id } => {
                self.retain_native_mouse_device(device_id, Some(false));
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::CursorLeft,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                let route = self.route_cursor_left();
                if let Some(routed) = finalize_native_immediate_transient_route(
                    self.complete_native_immediate_transient(ticket),
                    route.outcome,
                    route.launch_external_drag,
                    || self.launch_external_drag_if_armed(),
                ) {
                    self.handle_route_outcome(event_loop, routed);
                }
            }
            WindowEvent::MouseInput {
                device_id,
                button,
                state,
            } => {
                self.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::MouseInput,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let route =
                    self.route_native_mouse_input_with_timestamp(button, state, Some(timestamp));
                let Some(route_outcome) =
                    self.complete_native_discrete_input_route(ticket, route.outcome)
                else {
                    // The route already ran. Never replay it or apply a
                    // lower-stage fallback after a completion mismatch.
                    return;
                };
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
                    if should_request_native_maximize_redraw(route_outcome) {
                        self.request_redraw_for_frame_work(FrameWork::None);
                    }
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
                self.handle_route_outcome(event_loop, route_outcome);
            }
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {
                self.retain_native_mouse_device(device_id, None);
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::MouseWheel(phase),
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                let mut route =
                    self.route_native_mouse_wheel_with_phase_and_timestamp(delta, phase, timestamp);
                if let Some(outcome) =
                    self.complete_native_immediate_transient_route(ticket, route.outcome)
                {
                    route.outcome = outcome;
                    self.apply_native_mouse_wheel_route(route);
                    self.handle_route_outcome(event_loop, outcome);
                }
            }
            WindowEvent::Touch(touch) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_immediate_transient_event(
                    event_loop,
                    NativeImmediateTransientKind::Touch(touch.phase),
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let Some(ticket) =
                    self.revalidate_native_immediate_transient(ticket, adapter_generation, true)
                else {
                    return;
                };
                let modifiers = self.pointer_modifiers();
                if let Ok(sample) = normalize_touch(
                    &mut self.input.native_pointer_ingress,
                    touch,
                    self.window.dpi_scale,
                    modifiers,
                    timestamp,
                ) {
                    let _ = self.dispatch_native_touch_sample(sample);
                }
                let _ = self.complete_native_immediate_transient(ticket);
            }
            WindowEvent::PinchGesture {
                device_id,
                delta,
                phase,
            } => {
                self.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::PinchGesture(phase),
                    device_id,
                    GestureInput::Pinch { delta, phase },
                );
            }
            WindowEvent::RotationGesture {
                device_id,
                delta,
                phase,
            } => {
                self.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::RotationGesture(phase),
                    device_id,
                    GestureInput::Rotate {
                        delta_degrees: delta,
                        phase,
                    },
                );
            }
            WindowEvent::PanGesture {
                device_id,
                delta,
                phase,
            } => {
                self.normalize_native_gesture_transient(
                    event_loop,
                    NativeImmediateTransientKind::DesktopPanUnsupported(phase),
                    device_id,
                    GestureInput::Pan { delta, phase },
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::ModifiersChanged,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let routed = if self.should_launch_external_drag_before_app_switch(state) {
                    self.input.modifiers = state;
                    self.launch_external_drag_if_armed()
                } else {
                    self.route_native_modifiers_changed_with_timestamp(state, Some(timestamp))
                };
                if let Some(routed) = self.complete_native_discrete_input_route(ticket, routed) {
                    self.handle_route_outcome(event_loop, routed);
                }
            }
            WindowEvent::Ime(ime) => {
                let timestamp = InputTimestamp::capture();
                let Some(adapter_generation) = self
                    .adapter
                    .as_ref()
                    .and_then(GenericNativeAdapterOwner::capture_generation)
                else {
                    return;
                };
                let Some(ticket) = self.begin_native_discrete_input_event(
                    event_loop,
                    NativeDiscreteInputKind::Ime,
                    timestamp,
                    adapter_generation,
                    true,
                ) else {
                    return;
                };
                let routed = self.route_native_ime_event_with_timestamp(ime, Some(timestamp));
                if let Some(routed) = self.complete_native_discrete_input_route(ticket, routed) {
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
                self.handle_application_reopen_intent();
                if self.is_running() {
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
                    self.wake_native_surface_target_retirement_maintenance();
                    self.wake_normal_native_resource_maintenance();
                    if let Some(generation) = self
                        .adapter
                        .as_ref()
                        .and_then(GenericNativeAdapterOwner::capture_generation)
                    {
                        for window in &mut self.auxiliary_windows {
                            if window.is_admitted() {
                                window.wake_native_surface_target_retirement_maintenance();
                                window.wake_normal_native_resource_maintenance(generation);
                            }
                        }
                    }
                }
            }
            RuntimeUserEvent::NativeGpuTimingReady {
                route,
                generation,
                resource_identity,
                slot,
                token,
            } => self.handle_native_gpu_timing_ready(
                route,
                generation,
                resource_identity,
                slot,
                token,
            ),
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
        let current_generation = self
            .adapter
            .as_ref()
            .and_then(GenericNativeAdapterOwner::capture_generation);
        let mut maintenance = super::NativeResourceMaintenanceTurn::new();
        if let Some(current_generation) = current_generation {
            self.maintain_native_surface_target_retirement_if_due_with_turn(
                now,
                current_generation,
                &mut maintenance,
            );
            if let Some(index) = select_due_admitted_auxiliary_index(
                self.timing.auxiliary_surface_target_retirement_cursor,
                &self
                    .auxiliary_windows
                    .iter()
                    .map(|window| {
                        window.is_admitted()
                            && window
                                .native_surface_target_retirement_deadline()
                                .is_some_and(|deadline| deadline <= now)
                    })
                    .collect::<Vec<_>>(),
            ) {
                self.auxiliary_windows[index]
                    .maintain_native_surface_target_retirement_if_due_with_turn(
                        now,
                        current_generation,
                        &mut maintenance,
                    );
                self.timing.auxiliary_surface_target_retirement_cursor =
                    if self.auxiliary_windows.is_empty() {
                        0
                    } else {
                        (index + 1) % self.auxiliary_windows.len()
                    };
            }
        }
        let retiring_auxiliary_maintenance_due = self.retiring_auxiliary_maintenance_is_due(now);
        if retiring_auxiliary_maintenance_due {
            // One shared turn covers target-retirement and retiring-child
            // work. Ordinary maintenance is excluded specifically when
            // retiring-child cleanup is due.
            self.maintain_retiring_auxiliary_resources_with_turn(&mut maintenance);
            self.rearm_retiring_auxiliary_maintenance(now);
        }
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
            })
            .merge(FrameScheduleDeadlines {
                maintenance: self.native_surface_target_retirement_deadline(),
                ..FrameScheduleDeadlines::default()
            })
            .merge(FrameScheduleDeadlines {
                maintenance: self
                    .auxiliary_windows
                    .iter()
                    .filter_map(AuxiliaryNativeWindow::native_surface_target_retirement_deadline)
                    .min(),
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
                                &mut maintenance,
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
                            let admitted = self.adapter.as_mut().is_some_and(|adapter| {
                                self.auxiliary_windows
                                    .iter_mut()
                                    .find(|window| window.key() == key)
                                    .is_some_and(|window| {
                                        window.admit_native_resource_maintenance(
                                            adapter,
                                            now,
                                            &mut maintenance,
                                        )
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
                                native_discrete_input_route,
                                native_immediate_transient_route,
                            } = result;
                            debug_assert!(close_admission.is_none());
                            debug_assert!(native_discrete_input_route.is_none());
                            debug_assert!(native_immediate_transient_route.is_none());
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
                            let frame_gpu_timing =
                                if !shutdown_requested && terminal_cause.is_none() {
                                    self.auxiliary_windows
                                        .iter_mut()
                                        .find(|window| window.key() == key)
                                        .and_then(|window| window.take_ready_frame_gpu_timing())
                                } else {
                                    if let Some(window) = self
                                        .auxiliary_windows
                                        .iter_mut()
                                        .find(|window| window.key() == key)
                                    {
                                        window.discard_frame_gpu_timing();
                                    }
                                    None
                                };
                            forward_auxiliary_frame_diagnostics(self, &selected, frame_diagnostics);
                            forward_auxiliary_frame_gpu_timing(self, frame_gpu_timing);
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
                                    None,
                                    None,
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

fn forward_auxiliary_frame_gpu_timing<Bridge, Message>(
    runner: &mut GenericNativeVelloRunner<Bridge, Message>,
    handoff: Option<FrameGpuTimingSample>,
) where
    Bridge: RuntimeBridge<Message>,
{
    if let Some(sample) = handoff {
        runner.core.runtime.host_observe_frame_gpu_timing(sample);
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    fn handle_native_gpu_timing_ready(
        &mut self,
        route: NativeGpuTimingRoute,
        generation: super::NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
    ) {
        match route {
            NativeGpuTimingRoute::Primary => {
                match native_gpu_timing_ready_handling(self.native_lifecycle_snapshot()) {
                    NativeGpuTimingReadyHandling::Deliver => {
                        self.process_native_gpu_timing_ready(
                            generation,
                            resource_identity,
                            slot,
                            token,
                        );
                    }
                    NativeGpuTimingReadyHandling::Discard => {
                        self.discard_native_gpu_timing_ready(
                            generation,
                            resource_identity,
                            slot,
                            token,
                        );
                    }
                }
            }
            NativeGpuTimingRoute::Auxiliary(key) => {
                let parent_is_running = self.native_lifecycle_snapshot().is_running();
                let sample = self
                    .auxiliary_windows
                    .iter_mut()
                    .find(|window| window.key() == key)
                    .and_then(|window| {
                        if parent_is_running {
                            window.process_native_gpu_timing_ready(
                                generation,
                                resource_identity,
                                slot,
                                token,
                            )
                        } else {
                            window.discard_native_gpu_timing_ready(
                                generation,
                                resource_identity,
                                slot,
                                token,
                            );
                            None
                        }
                    });
                forward_auxiliary_frame_gpu_timing(self, sample);
            }
        }
    }

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
        NativeGpuTimingReadyHandling, NativeLifecycle, forward_auxiliary_frame_diagnostics,
        native_gpu_timing_ready_handling, native_resource_maintenance_deadline,
    };
    use crate::gui_runtime::native_vello::generic_runtime::cpu_frame_fairness::{
        CpuFrameCadencePressure, CpuFrameCadenceRate,
    };
    use crate::gui_runtime::native_vello::generic_runtime::cpu_frame_observation::CpuFrameObservationCapture;
    use crate::gui_runtime::native_vello::generic_runtime::frame_scheduler_policy::NativeInputStageDisposition;
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
            WindowEvent::Occluded(false),
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
    fn exceeded_titlebar_maximize_suppresses_only_native_redraw_request() {
        let deferred = super::GenericRouteOutcome::default()
            .with_native_input_stage_disposition(NativeInputStageDisposition::DeferLowerPriority);
        let continued = super::GenericRouteOutcome::default()
            .with_native_input_stage_disposition(NativeInputStageDisposition::ContinueNow);

        assert!(!super::should_request_native_maximize_redraw(deferred));
        assert!(super::should_request_native_maximize_redraw(continued));
        assert!(super::should_request_native_maximize_redraw(
            super::GenericRouteOutcome::default()
        ));
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

    #[test]
    fn gpu_timing_completion_is_delivered_only_while_running() {
        assert_eq!(
            native_gpu_timing_ready_handling(NativeLifecycle::default()),
            NativeGpuTimingReadyHandling::Deliver
        );

        let mut recovering = NativeLifecycle::default();
        assert!(recovering.admit_recovery());
        assert_eq!(
            native_gpu_timing_ready_handling(recovering),
            NativeGpuTimingReadyHandling::Discard
        );

        let mut closing = NativeLifecycle::default();
        assert!(closing.admit_closing(Instant::now()));
        assert_eq!(
            native_gpu_timing_ready_handling(closing),
            NativeGpuTimingReadyHandling::Discard
        );

        assert_eq!(
            native_gpu_timing_ready_handling(NativeLifecycle::Stopped),
            NativeGpuTimingReadyHandling::Discard
        );
    }
}
