//! Native pointer routing contract for the generic native Vello runner.

use super::gpu_surface_wheel::DeferredWheelRouteEffects;
use super::input::NativePointerGestureLatch;
use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, is_double_click, maybe_log_route_profile,
    native_pointer_press_gesture, native_wheel_sample, pointer_button_from_winit,
    pointer_modifiers_for_native_gesture, pointer_modifiers_from_winit, render_profile_enabled,
    scroll_delta_to_logical,
};
use crate::{
    gui::input::InputTimestamp,
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers},
};
use std::time::Instant;
use tracing::debug;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta, TouchPhase},
    keyboard::ModifiersState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePointerEventKind {
    MousePress,
    MouseRelease,
    MouseWheel,
    ModifiersChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativePointerRouteResult {
    NoCursor,
    UnsupportedButton,
    Coalesced,
    Routed,
    Unrouted,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct NativePointerRouteDiagnostic {
    pub(super) kind: NativePointerEventKind,
    pub(super) position: Option<Point>,
    pub(super) button: Option<PointerButton>,
    pub(super) modifiers: PointerModifiers,
    pub(super) hit_target: Option<crate::widgets::WidgetId>,
    pub(super) captured_widget: Option<crate::widgets::WidgetId>,
    pub(super) result: NativePointerRouteResult,
    pub(super) outcome: GenericRouteOutcome,
    pub(super) deferred_surface_refresh: bool,
    pub(super) deferred_scene_rebuild: bool,
    pub(super) pending_viewport_resize: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeMouseInputRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) position: Option<Point>,
    pub(super) button: Option<PointerButton>,
    pub(super) state: ElementState,
    pub(super) double_click: bool,
    #[cfg(test)]
    pub(super) diagnostic: NativePointerRouteDiagnostic,
}

impl NativeMouseInputRoute {
    fn new(
        outcome: GenericRouteOutcome,
        position: Option<Point>,
        button: Option<PointerButton>,
        state: ElementState,
        double_click: bool,
        _diagnostic: NativePointerRouteDiagnostic,
    ) -> Self {
        Self {
            outcome,
            position,
            button,
            state,
            double_click,
            #[cfg(test)]
            diagnostic: _diagnostic,
        }
    }

    pub(super) fn is_pressed(self) -> bool {
        self.state == ElementState::Pressed
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeWheelRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) position: Option<Point>,
    pub(super) delta: crate::gui::types::Vector2,
    pub(super) deferred_wheel_effects: DeferredWheelRouteEffects,
    pub(super) redraw_requested: bool,
    #[cfg(test)]
    pub(super) diagnostic: NativePointerRouteDiagnostic,
}

impl NativeWheelRoute {
    fn new(
        outcome: GenericRouteOutcome,
        position: Option<Point>,
        delta: crate::gui::types::Vector2,
        deferred_wheel_effects: DeferredWheelRouteEffects,
        redraw_requested: bool,
        _diagnostic: NativePointerRouteDiagnostic,
    ) -> Self {
        Self {
            outcome,
            position,
            delta,
            deferred_wheel_effects,
            redraw_requested,
            #[cfg(test)]
            diagnostic: _diagnostic,
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    #[cfg(test)]
    pub(super) fn route_native_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> NativeMouseInputRoute {
        let timestamp = (state == ElementState::Pressed || state == ElementState::Released)
            .then(InputTimestamp::capture);
        self.route_native_mouse_input_with_timestamp(button, state, timestamp)
    }

    pub(super) fn route_native_mouse_input_with_timestamp(
        &mut self,
        button: MouseButton,
        state: ElementState,
        timestamp: Option<InputTimestamp>,
    ) -> NativeMouseInputRoute {
        let kind = match state {
            ElementState::Pressed => NativePointerEventKind::MousePress,
            ElementState::Released => NativePointerEventKind::MouseRelease,
        };
        let position = self.input.last_cursor;
        let physical_button = pointer_button_from_winit(button);
        let latched_gesture = self
            .input
            .effective_pointer_gesture
            .filter(|latch| latch.physical_button == button);
        let gesture = match state {
            ElementState::Pressed => {
                let gesture = native_pointer_press_gesture(physical_button, self.input.modifiers);
                if let Some(gesture) = gesture.filter(|gesture| gesture.consume_control) {
                    self.input.effective_pointer_gesture = Some(NativePointerGestureLatch {
                        physical_button: button,
                        gesture,
                    });
                }
                gesture
            }
            ElementState::Released => latched_gesture.map(|latch| latch.gesture).or_else(|| {
                physical_button.map(|button| super::input::NativePointerGesture {
                    button,
                    consume_control: false,
                })
            }),
        };
        let button = gesture.map(|gesture| gesture.button);
        let modifiers = gesture.map_or_else(
            || self.pointer_modifiers(),
            |gesture| self.pointer_modifiers_for_gesture(gesture.consume_control),
        );
        let double_click = state == ElementState::Pressed
            && self.core.last_pointer_press.is_some_and(|last| {
                position.is_some_and(|position| {
                    button.is_some_and(|button| {
                        is_double_click(last, Instant::now(), position, button)
                    })
                })
            });
        let mut diagnostic = self.native_pointer_diagnostic(kind, position, button, modifiers);
        let Some(position) = position else {
            diagnostic.result = NativePointerRouteResult::NoCursor;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            let route = NativeMouseInputRoute::new(
                GenericRouteOutcome::default(),
                None,
                button,
                state,
                double_click,
                diagnostic,
            );
            if latched_gesture.is_some() {
                self.input.effective_pointer_gesture = None;
            }
            return route;
        };
        let Some(button) = button else {
            diagnostic.result = NativePointerRouteResult::UnsupportedButton;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            let route = NativeMouseInputRoute::new(
                GenericRouteOutcome::default(),
                Some(position),
                None,
                state,
                double_click,
                diagnostic,
            );
            if latched_gesture.is_some() {
                self.input.effective_pointer_gesture = None;
            }
            return route;
        };
        if state == ElementState::Released {
            self.flush_pending_scrollbar_drag_now();
        }
        self.flush_pending_wheel_input_now();
        let modifiers = gesture.map_or_else(
            || self.pointer_modifiers(),
            |gesture| self.pointer_modifiers_for_gesture(gesture.consume_control),
        );
        let started = Instant::now();
        let outcome = match state {
            ElementState::Pressed => self
                .core
                .route_pointer_press_with_timestamp(position, button, modifiers, timestamp),
            ElementState::Released => self
                .core
                .route_pointer_release_with_timestamp(position, button, modifiers, timestamp),
        };
        maybe_log_route_profile("pointer_button", started.elapsed(), outcome);
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        let route = NativeMouseInputRoute::new(
            outcome,
            Some(position),
            Some(button),
            state,
            double_click,
            diagnostic,
        );
        if latched_gesture.is_some() {
            self.input.effective_pointer_gesture = None;
        }
        route
    }

    #[cfg(test)]
    pub(super) fn route_native_mouse_wheel(&mut self, delta: MouseScrollDelta) -> NativeWheelRoute {
        self.route_native_mouse_wheel_internal(delta, None, None, true)
    }

    #[cfg(test)]
    pub(super) fn route_native_mouse_wheel_with_phase(
        &mut self,
        raw_delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> NativeWheelRoute {
        self.route_native_mouse_wheel_internal(raw_delta, Some(phase), None, true)
    }

    /// Route one wheel event for the ImmediateTransient stage. Lower-stage
    /// GPU/redraw work is applied by the event owner after ticket completion.
    pub(super) fn route_native_mouse_wheel_with_phase_and_timestamp(
        &mut self,
        raw_delta: MouseScrollDelta,
        phase: TouchPhase,
        timestamp: InputTimestamp,
    ) -> NativeWheelRoute {
        self.route_native_mouse_wheel_internal(raw_delta, Some(phase), Some(timestamp), false)
    }

    fn route_native_mouse_wheel_internal(
        &mut self,
        raw_delta: MouseScrollDelta,
        phase: Option<TouchPhase>,
        captured_timestamp: Option<InputTimestamp>,
        apply_route_effects: bool,
    ) -> NativeWheelRoute {
        let timestamp = captured_timestamp.or_else(|| Some(InputTimestamp::capture()));
        let position = self.input.last_cursor;
        let delta = scroll_delta_to_logical(raw_delta, self.window.dpi_scale);
        let consume_control = self
            .input
            .effective_pointer_gesture
            .is_some_and(|latch| latch.gesture.consume_control);
        let modifiers = self.pointer_modifiers_for_gesture(consume_control);
        let now = Instant::now();
        let lifecycle_boundary = phase.is_some_and(|phase| {
            matches!(
                phase,
                TouchPhase::Started | TouchPhase::Ended | TouchPhase::Cancelled
            )
        });
        let mut deferred_wheel_effects = if lifecycle_boundary {
            if apply_route_effects {
                self.flush_pending_wheel_input_now();
                DeferredWheelRouteEffects::default()
            } else {
                self.route_pending_wheel_input_for_immediate_transient()
            }
        } else {
            if self.pending_interactive_scroll_flush_is_due(now) {
                if apply_route_effects {
                    self.flush_stale_pending_wheel_input(now);
                    DeferredWheelRouteEffects::default()
                } else {
                    self.route_pending_wheel_input_for_immediate_transient()
                }
            } else {
                DeferredWheelRouteEffects::default()
            }
        };
        let mut redraw_requested = false;
        let mut diagnostic = self.native_pointer_diagnostic(
            NativePointerEventKind::MouseWheel,
            position,
            None,
            modifiers,
        );
        let Some(position) = position else {
            diagnostic.result = NativePointerRouteResult::NoCursor;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(
                GenericRouteOutcome::default(),
                None,
                delta,
                deferred_wheel_effects,
                redraw_requested,
                diagnostic,
            );
        };
        let sequence_range = self.input.input_sequence_allocator.allocate();
        let exact_sample = phase.map(|phase| {
            native_wheel_sample(
                raw_delta,
                phase,
                self.window.dpi_scale,
                modifiers,
                timestamp,
                sequence_range,
            )
        });
        if phase.is_none() && self.can_coalesce_gpu_surface_wheel(position, delta) {
            let queued_effects = if apply_route_effects {
                self.queue_gpu_surface_wheel_with_metadata(
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                );
                DeferredWheelRouteEffects::default()
            } else {
                self.queue_gpu_surface_wheel_with_metadata_for_immediate_transient(
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                )
            };
            deferred_wheel_effects.merge(queued_effects);
            redraw_requested = true;
            let outcome = GenericRouteOutcome::default();
            diagnostic.result = NativePointerRouteResult::Coalesced;
            diagnostic.outcome = outcome;
            diagnostic.deferred_surface_refresh = self.timing.deferred_surface_refresh;
            diagnostic.deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(
                outcome,
                Some(position),
                delta,
                deferred_wheel_effects,
                redraw_requested,
                diagnostic,
            );
        }
        let can_queue_scroll_container_wheel = match exact_sample.as_ref() {
            Some(Ok(sample)) => self
                .core
                .runtime
                .can_coalesce_scroll_container_wheel_with_sample(position, *sample),
            Some(Err(_)) => false,
            None => self.can_coalesce_scroll_container_wheel_with_timestamp(
                position, delta, modifiers, timestamp,
            ),
        } && !self
            .pending_interactive_scroll_flush_is_due(now);
        if can_queue_scroll_container_wheel {
            let queued_effects = if apply_route_effects {
                self.queue_scroll_container_wheel_with_metadata(
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                );
                DeferredWheelRouteEffects::default()
            } else {
                self.queue_scroll_container_wheel_with_metadata_for_immediate_transient(
                    position,
                    delta,
                    modifiers,
                    timestamp,
                    sequence_range,
                )
            };
            deferred_wheel_effects.merge(queued_effects);
            redraw_requested = true;
            let outcome = GenericRouteOutcome::default();
            diagnostic.result = NativePointerRouteResult::Coalesced;
            diagnostic.outcome = outcome;
            diagnostic.deferred_surface_refresh = self.timing.deferred_surface_refresh;
            diagnostic.deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(
                outcome,
                Some(position),
                delta,
                deferred_wheel_effects,
                redraw_requested,
                diagnostic,
            );
        }
        let started = Instant::now();
        let outcome = match exact_sample {
            Some(Ok(sample)) => self
                .core
                .route_scroll_deferred_refresh_with_sample(position, sample),
            _ => self.core.route_scroll_deferred_refresh_with_metadata(
                position,
                delta,
                modifiers,
                timestamp,
                sequence_range,
            ),
        };
        maybe_log_route_profile("wheel", started.elapsed(), outcome);
        if apply_route_effects {
            self.handle_gpu_surface_route_outcome(outcome, position, delta);
        }
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        NativeWheelRoute::new(
            outcome,
            Some(position),
            delta,
            deferred_wheel_effects,
            redraw_requested,
            diagnostic,
        )
    }

    fn flush_stale_pending_wheel_input(&mut self, now: Instant) {
        if !self.pending_interactive_scroll_flush_is_due(now) {
            return;
        }
        let mut profile = super::RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.flush_pending_scroll_container_wheel(&mut profile);
    }

    #[cfg(test)]
    pub(super) fn route_native_modifiers_changed(
        &mut self,
        modifiers: ModifiersState,
    ) -> GenericRouteOutcome {
        let timestamp = Some(InputTimestamp::capture());
        self.route_native_modifiers_changed_with_timestamp(modifiers, timestamp)
    }

    pub(super) fn route_native_modifiers_changed_with_timestamp(
        &mut self,
        modifiers: ModifiersState,
        timestamp: Option<InputTimestamp>,
    ) -> GenericRouteOutcome {
        self.input.modifiers = modifiers;
        let consume_control = self
            .input
            .effective_pointer_gesture
            .is_some_and(|latch| latch.gesture.consume_control);
        let mut diagnostic = self.native_pointer_diagnostic(
            NativePointerEventKind::ModifiersChanged,
            self.input.last_cursor,
            None,
            self.pointer_modifiers_for_gesture(consume_control),
        );
        let outcome = self.core.route_pointer_modifiers_changed(
            pointer_modifiers_for_native_gesture(modifiers, consume_control),
            timestamp,
        );
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        outcome
    }

    pub(super) fn pointer_modifiers(&self) -> PointerModifiers {
        pointer_modifiers_from_winit(self.input.modifiers)
    }

    fn pointer_modifiers_for_gesture(&self, consume_control: bool) -> PointerModifiers {
        pointer_modifiers_for_native_gesture(self.input.modifiers, consume_control)
    }

    fn native_pointer_diagnostic(
        &self,
        kind: NativePointerEventKind,
        position: Option<Point>,
        button: Option<PointerButton>,
        modifiers: PointerModifiers,
    ) -> NativePointerRouteDiagnostic {
        NativePointerRouteDiagnostic {
            kind,
            position,
            button,
            modifiers,
            hit_target: position.and_then(|position| self.core.runtime.widget_at(position)),
            captured_widget: self.core.runtime.pointer_capture(),
            result: NativePointerRouteResult::Unrouted,
            outcome: GenericRouteOutcome::default(),
            deferred_surface_refresh: self.timing.deferred_surface_refresh,
            deferred_scene_rebuild: self.timing.deferred_scene_rebuild,
            pending_viewport_resize: self.timing.pending_viewport_resize.is_some(),
        }
    }

    fn complete_native_pointer_diagnostic(
        &self,
        mut diagnostic: NativePointerRouteDiagnostic,
        outcome: GenericRouteOutcome,
    ) -> NativePointerRouteDiagnostic {
        diagnostic.result = if outcome.routed {
            NativePointerRouteResult::Routed
        } else {
            NativePointerRouteResult::Unrouted
        };
        diagnostic.outcome = outcome;
        diagnostic.deferred_surface_refresh = self.timing.deferred_surface_refresh;
        diagnostic.deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
        diagnostic.pending_viewport_resize = self.timing.pending_viewport_resize.is_some();
        diagnostic
    }

    fn maybe_log_native_pointer_diagnostic(&self, diagnostic: NativePointerRouteDiagnostic) {
        if !render_profile_enabled() {
            return;
        }
        debug!(?diagnostic, "radiant native pointer route");
    }
}
