use super::super::*;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::ModifiersState,
};

pub(in super::super) struct NativePointerHarness<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in super::super) runner: GenericNativeVelloRunner<Bridge, Message>,
}

impl<Bridge, Message> NativePointerHarness<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in super::super) fn new(bridge: Bridge, viewport: Vector2) -> Self {
        let mut runner =
            GenericNativeVelloRunner::new(NativeRunOptions::default(), bridge, viewport);
        runner.rebuild_scene();
        Self { runner }
    }

    pub(in super::super) fn cursor_moved_logical(&mut self, position: Point) {
        let physical = PhysicalPosition::new(
            self.runner.window.dpi_scale.logical_to_physical(position.x) as f64,
            self.runner.window.dpi_scale.logical_to_physical(position.y) as f64,
        );
        self.cursor_moved_physical(physical);
    }

    pub(in super::super) fn cursor_moved_physical(&mut self, position: PhysicalPosition<f64>) {
        self.runner.handle_cursor_moved(position);
    }

    pub(in super::super) fn modifiers_changed(&mut self, modifiers: ModifiersState) {
        self.runner.input.modifiers = modifiers;
        let outcome = self
            .runner
            .core
            .route_pointer_modifiers_changed(pointer_modifiers_from_winit(modifiers));
        self.apply_route_outcome(outcome);
    }

    pub(in super::super) fn mouse_pressed(&mut self, button: MouseButton) -> GenericRouteOutcome {
        self.mouse_input(button, ElementState::Pressed)
    }

    pub(in super::super) fn mouse_released(&mut self, button: MouseButton) -> GenericRouteOutcome {
        self.mouse_input(button, ElementState::Released)
    }

    pub(in super::super) fn mouse_wheel(&mut self, delta: MouseScrollDelta) -> GenericRouteOutcome {
        let Some(position) = self.runner.input.last_cursor else {
            return GenericRouteOutcome::default();
        };
        let delta = scroll_delta_to_logical(delta, self.runner.window.dpi_scale);
        let modifiers = pointer_modifiers_from_winit(self.runner.input.modifiers);
        if self.runner.can_coalesce_gpu_surface_wheel(position, delta) {
            self.runner
                .queue_gpu_surface_wheel(position, delta, modifiers);
            return GenericRouteOutcome {
                paint_only_requested: true,
                ..GenericRouteOutcome::default()
            };
        }
        let outcome = if self.runner.can_fast_path_gpu_surface_route(position, delta) {
            self.runner
                .core
                .route_scroll_deferred_refresh_with_modifiers(position, delta, modifiers)
        } else {
            self.runner
                .core
                .route_scroll_with_modifiers(position, delta, modifiers)
        };
        self.runner
            .handle_gpu_surface_route_outcome(outcome, position, delta);
        outcome
    }

    pub(in super::super) fn focus_lost(&mut self) -> GenericRouteOutcome {
        self.runner.handle_focus_lost_before_external_drag()
    }

    pub(in super::super) fn focus_regained(&mut self) {
        self.runner.handle_focus_regained_after_native_modal_loop();
    }

    fn mouse_input(&mut self, button: MouseButton, state: ElementState) -> GenericRouteOutcome {
        let Some(position) = self.runner.input.last_cursor else {
            return GenericRouteOutcome::default();
        };
        let Some(button) = pointer_button_from_winit(button) else {
            return GenericRouteOutcome::default();
        };
        let modifiers = pointer_modifiers_from_winit(self.runner.input.modifiers);
        let outcome = match state {
            ElementState::Pressed => self
                .runner
                .core
                .route_pointer_press_with_modifiers(position, button, modifiers),
            ElementState::Released => self
                .runner
                .core
                .route_pointer_release_with_modifiers(position, button, modifiers),
        };
        self.apply_route_outcome(outcome);
        outcome
    }

    fn apply_route_outcome(&mut self, mut outcome: GenericRouteOutcome) {
        self.runner.merge_due_timed_frame_for_route(&mut outcome);
        if outcome.interactive_surface_refresh_requested {
            self.runner
                .refresh_and_rebuild_scene_for_interactive_route_now();
        } else if outcome.interactive_scene_rebuild_requested {
            self.runner.rebuild_scene_for_interactive_route_now();
        } else if outcome.needs_scene_rebuild() {
            self.runner.rebuild_scene();
        } else if outcome.deferred_surface_refresh_requested {
            self.runner.timing.deferred_surface_refresh = true;
        }
    }
}
