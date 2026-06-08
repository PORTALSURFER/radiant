//! Native pointer routing contract for the generic native Vello runner.

use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, maybe_log_route_profile,
    pointer_button_from_winit, pointer_modifiers_from_winit, scroll_delta_to_logical,
};
use crate::{
    gui::types::Point,
    runtime::RuntimeBridge,
    widgets::{PointerButton, PointerModifiers},
};
use std::time::Instant;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::ModifiersState,
};

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeMouseInputRoute {
    pub(super) outcome: GenericRouteOutcome,
    pub(super) position: Point,
    pub(super) button: PointerButton,
    pub(super) state: ElementState,
}

impl NativeMouseInputRoute {
    pub(super) fn is_pressed(self) -> bool {
        self.state == ElementState::Pressed
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
    ) -> Option<NativeMouseInputRoute> {
        let position = self.input.last_cursor?;
        let button = pointer_button_from_winit(button)?;
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
        Some(NativeMouseInputRoute {
            outcome,
            position,
            button,
            state,
        })
    }

    pub(super) fn route_native_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
    ) -> Option<GenericRouteOutcome> {
        let position = self.input.last_cursor?;
        let delta = scroll_delta_to_logical(delta, self.window.dpi_scale);
        let modifiers = self.pointer_modifiers();
        if self.can_coalesce_gpu_surface_wheel(position, delta) {
            self.queue_gpu_surface_wheel(position, delta, modifiers);
            return Some(GenericRouteOutcome {
                paint_only_requested: true,
                ..GenericRouteOutcome::default()
            });
        }
        let started = Instant::now();
        let outcome = if self.can_fast_path_gpu_surface_route(position, delta) {
            self.core
                .route_scroll_deferred_refresh_with_modifiers(position, delta, modifiers)
        } else {
            self.core
                .route_scroll_with_modifiers(position, delta, modifiers)
        };
        maybe_log_route_profile("wheel", started.elapsed(), outcome);
        self.handle_gpu_surface_route_outcome(outcome, position, delta);
        Some(outcome)
    }

    pub(super) fn route_native_modifiers_changed(
        &mut self,
        modifiers: ModifiersState,
    ) -> GenericRouteOutcome {
        self.input.modifiers = modifiers;
        self.core
            .route_pointer_modifiers_changed(pointer_modifiers_from_winit(modifiers))
    }

    fn pointer_modifiers(&self) -> PointerModifiers {
        pointer_modifiers_from_winit(self.input.modifiers)
    }
}
