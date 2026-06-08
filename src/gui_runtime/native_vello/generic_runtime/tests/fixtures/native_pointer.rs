use super::super::*;
use winit::{dpi::PhysicalPosition, event::MouseButton, keyboard::ModifiersState};

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
        let outcome = self.runner.route_native_modifiers_changed(modifiers);
        self.apply_route_outcome(outcome);
    }

    pub(in super::super) fn mouse_pressed(&mut self, button: MouseButton) -> GenericRouteOutcome {
        let route = self
            .runner
            .route_native_mouse_input(button, winit::event::ElementState::Pressed);
        let outcome = route.outcome;
        self.apply_route_outcome(outcome);
        outcome
    }

    pub(in super::super) fn mouse_released(&mut self, button: MouseButton) -> GenericRouteOutcome {
        let route = self
            .runner
            .route_native_mouse_input(button, winit::event::ElementState::Released);
        let outcome = route.outcome;
        self.apply_route_outcome(outcome);
        outcome
    }

    pub(in super::super) fn mouse_pressed_route(
        &mut self,
        button: MouseButton,
    ) -> NativeMouseInputRoute {
        let route = self
            .runner
            .route_native_mouse_input(button, winit::event::ElementState::Pressed);
        self.apply_route_outcome(route.outcome);
        route
    }

    pub(in super::super) fn mouse_released_route(
        &mut self,
        button: MouseButton,
    ) -> NativeMouseInputRoute {
        let route = self
            .runner
            .route_native_mouse_input(button, winit::event::ElementState::Released);
        self.apply_route_outcome(route.outcome);
        route
    }

    pub(in super::super) fn mouse_wheel_route(
        &mut self,
        delta: winit::event::MouseScrollDelta,
    ) -> NativeWheelRoute {
        self.runner.route_native_mouse_wheel(delta)
    }

    pub(in super::super) fn focus_regained(&mut self) {
        self.runner.handle_focus_regained_after_native_modal_loop();
    }

    pub(in super::super) fn focus_lost(&mut self) -> GenericRouteOutcome {
        self.runner.handle_focus_lost_before_external_drag()
    }

    fn apply_route_outcome(&mut self, outcome: GenericRouteOutcome) {
        self.runner.apply_route_outcome(outcome);
    }
}
