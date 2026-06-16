//! Native pointer routing contract for the generic native Vello runner.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, maybe_log_route_profile,
    pointer_button_from_winit, pointer_modifiers_from_winit, render_profile_enabled,
    scroll_delta_to_logical,
};
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers},
};
use std::time::Instant;
use tracing::debug;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
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
    #[cfg(test)]
    pub(super) diagnostic: NativePointerRouteDiagnostic,
}

impl NativeMouseInputRoute {
    fn new(
        outcome: GenericRouteOutcome,
        position: Option<Point>,
        button: Option<PointerButton>,
        state: ElementState,
        _diagnostic: NativePointerRouteDiagnostic,
    ) -> Self {
        Self {
            outcome,
            position,
            button,
            state,
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
    #[cfg(test)]
    pub(super) outcome: GenericRouteOutcome,
    #[cfg(test)]
    pub(super) diagnostic: NativePointerRouteDiagnostic,
}

impl NativeWheelRoute {
    fn new(_outcome: GenericRouteOutcome, _diagnostic: NativePointerRouteDiagnostic) -> Self {
        Self {
            #[cfg(test)]
            outcome: _outcome,
            #[cfg(test)]
            diagnostic: _diagnostic,
        }
    }
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn route_native_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
    ) -> NativeMouseInputRoute {
        let kind = match state {
            ElementState::Pressed => NativePointerEventKind::MousePress,
            ElementState::Released => NativePointerEventKind::MouseRelease,
        };
        let position = self.input.last_cursor;
        let button = pointer_button_from_winit(button);
        let modifiers = self.pointer_modifiers();
        let mut diagnostic = self.native_pointer_diagnostic(kind, position, button, modifiers);
        let Some(position) = position else {
            diagnostic.result = NativePointerRouteResult::NoCursor;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeMouseInputRoute::new(
                GenericRouteOutcome::default(),
                None,
                button,
                state,
                diagnostic,
            );
        };
        let Some(button) = button else {
            diagnostic.result = NativePointerRouteResult::UnsupportedButton;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeMouseInputRoute::new(
                GenericRouteOutcome::default(),
                Some(position),
                None,
                state,
                diagnostic,
            );
        };
        if state == ElementState::Released {
            self.flush_pending_scrollbar_drag_now();
        }
        let modifiers = self.pointer_modifiers();
        let started = Instant::now();
        let outcome = match state {
            ElementState::Pressed => self
                .core
                .route_pointer_press_with_modifiers(position, button, modifiers),
            ElementState::Released => self
                .core
                .route_pointer_release_with_modifiers(position, button, modifiers),
        };
        maybe_log_route_profile("pointer_button", started.elapsed(), outcome);
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        NativeMouseInputRoute::new(outcome, Some(position), Some(button), state, diagnostic)
    }

    pub(super) fn route_native_mouse_wheel(&mut self, delta: MouseScrollDelta) -> NativeWheelRoute {
        let position = self.input.last_cursor;
        let delta = scroll_delta_to_logical(delta, self.window.dpi_scale);
        let modifiers = self.pointer_modifiers();
        let now = Instant::now();
        self.flush_stale_pending_wheel_input(now);
        let mut diagnostic = self.native_pointer_diagnostic(
            NativePointerEventKind::MouseWheel,
            position,
            None,
            modifiers,
        );
        let Some(position) = position else {
            diagnostic.result = NativePointerRouteResult::NoCursor;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(GenericRouteOutcome::default(), diagnostic);
        };
        if self.can_coalesce_gpu_surface_wheel(position, delta) {
            self.queue_gpu_surface_wheel(position, delta, modifiers);
            let outcome = GenericRouteOutcome {
                paint_only_requested: true,
                ..GenericRouteOutcome::default()
            };
            diagnostic.result = NativePointerRouteResult::Coalesced;
            diagnostic.outcome = outcome;
            diagnostic.deferred_surface_refresh = self.timing.deferred_surface_refresh;
            diagnostic.deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(outcome, diagnostic);
        }
        let can_queue_scroll_container_wheel = self
            .can_coalesce_scroll_container_wheel(position, delta)
            && self.timing.redraw_requested
            && !self.pending_interactive_scroll_flush_is_due(now);
        if can_queue_scroll_container_wheel {
            self.queue_scroll_container_wheel(position, delta, modifiers);
            let outcome = GenericRouteOutcome {
                paint_only_requested: true,
                ..GenericRouteOutcome::default()
            };
            diagnostic.result = NativePointerRouteResult::Coalesced;
            diagnostic.outcome = outcome;
            diagnostic.deferred_surface_refresh = self.timing.deferred_surface_refresh;
            diagnostic.deferred_scene_rebuild = self.timing.deferred_scene_rebuild;
            self.maybe_log_native_pointer_diagnostic(diagnostic);
            return NativeWheelRoute::new(outcome, diagnostic);
        }
        let started = Instant::now();
        let outcome = self
            .core
            .route_scroll_deferred_refresh_with_modifiers(position, delta, modifiers);
        maybe_log_route_profile("wheel", started.elapsed(), outcome);
        self.handle_gpu_surface_route_outcome(outcome, position, delta);
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        NativeWheelRoute::new(outcome, diagnostic)
    }

    fn flush_stale_pending_wheel_input(&mut self, now: Instant) {
        if !self.pending_interactive_scroll_flush_is_due(now) {
            return;
        }
        let mut profile = super::RenderFrameProfile::default();
        self.flush_pending_gpu_surface_wheel(&mut profile);
        self.flush_pending_scroll_container_wheel(&mut profile);
    }

    pub(super) fn route_native_modifiers_changed(
        &mut self,
        modifiers: ModifiersState,
    ) -> GenericRouteOutcome {
        self.input.modifiers = modifiers;
        let mut diagnostic = self.native_pointer_diagnostic(
            NativePointerEventKind::ModifiersChanged,
            self.input.last_cursor,
            None,
            self.pointer_modifiers(),
        );
        let outcome = self
            .core
            .route_pointer_modifiers_changed(pointer_modifiers_from_winit(modifiers));
        diagnostic = self.complete_native_pointer_diagnostic(diagnostic, outcome);
        self.maybe_log_native_pointer_diagnostic(diagnostic);
        outcome
    }

    fn pointer_modifiers(&self) -> PointerModifiers {
        pointer_modifiers_from_winit(self.input.modifiers)
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
