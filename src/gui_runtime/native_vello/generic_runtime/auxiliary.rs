use super::{
    GenericNativeVelloRunner, GenericRouteOutcome, initial_viewport, owner_window_handle,
    pointer_button_from_winit, pointer_modifiers_from_winit, scroll_delta_to_logical,
};
use crate::runtime::{AuxiliaryWindow, NativeRunOptions, RuntimeBridge};
use bridge::AuxiliarySurfaceBridge;
use placement::centered_position;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

mod bridge;
mod placement;

pub(super) struct AuxiliaryNativeWindow<Message> {
    key: String,
    close_message: Option<Message>,
    runner: GenericNativeVelloRunner<AuxiliarySurfaceBridge<Message>, Message>,
    active: bool,
}

impl<Message> AuxiliaryNativeWindow<Message> {
    pub(super) fn new(
        projection: AuxiliaryWindow<Message>,
        parent_options: &NativeRunOptions,
    ) -> Self {
        let viewport = initial_viewport(&projection.options);
        let mut options = projection.options;
        options.frame.debug_layout |= parent_options.frame.debug_layout;
        if options.text.embedded_fonts.is_empty() && options.text.font_paths.is_empty() {
            options.text = parent_options.text.clone();
        }
        let bridge = AuxiliarySurfaceBridge::new(projection.surface);
        Self {
            key: projection.key,
            close_message: projection.close_message,
            runner: GenericNativeVelloRunner::new(options, bridge, viewport),
            active: true,
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    pub(super) fn window_id(&self) -> Option<WindowId> {
        self.runner.window.id
    }

    pub(super) fn update_projection(&mut self, projection: AuxiliaryWindow<Message>) {
        self.close_message = projection.close_message;
        self.runner.core.runtime.bridge_mut().surface = projection.surface;
        self.runner.core.refresh_surface();
        self.runner.rebuild_scene();
        self.show();
        self.runner.request_redraw_if_needed();
    }

    pub(super) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
        parent_window: Option<&Window>,
    ) {
        if self
            .runner
            .options
            .window
            .behavior
            .owner_window_handle
            .is_none()
        {
            self.runner.options.window.behavior.owner_window_handle =
                owner_window_handle(parent_window);
        }
        if self.runner.options.window.geometry.position.is_none() {
            self.runner.options.window.geometry.position =
                centered_position(parent_window, &self.runner.options);
        }
        self.runner.initialize_runtime(event_loop);
    }

    pub(super) fn hide(&mut self) {
        self.active = false;
        if let Some(window) = self.runner.window.window.as_ref() {
            window.set_visible(false);
        }
    }

    pub(super) fn show(&mut self) {
        self.active = true;
        if let Some(window) = self.runner.window.window.as_ref() {
            window.set_visible(true);
            window.focus_window();
        }
    }

    pub(super) fn route_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> AuxiliaryWindowEventResult<Message> {
        match event {
            WindowEvent::CloseRequested => {
                return AuxiliaryWindowEventResult {
                    closed: true,
                    messages: self.close_message.take().into_iter().collect(),
                };
            }
            WindowEvent::Resized(size) => self.runner.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runner.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::CursorMoved { position, .. } => self.runner.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.runner.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                self.route_mouse_input(event_loop, button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => self.route_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                self.runner.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.runner.input.modifiers = modifiers.state()
            }
            WindowEvent::RedrawRequested => self.runner.redraw(event_loop),
            _ => {}
        }
        AuxiliaryWindowEventResult {
            closed: false,
            messages: self.take_messages(),
        }
    }

    fn route_mouse_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        button: winit::event::MouseButton,
        state: ElementState,
    ) {
        let Some(position) = self.runner.input.last_cursor else {
            return;
        };
        let Some(button) = pointer_button_from_winit(button) else {
            return;
        };
        let modifiers = pointer_modifiers_from_winit(self.runner.input.modifiers);
        let routed = match state {
            ElementState::Pressed => self
                .runner
                .core
                .route_pointer_press_with_modifiers(position, button, modifiers),
            ElementState::Released => self
                .runner
                .core
                .route_pointer_release_with_modifiers(position, button, modifiers),
        };
        self.runner.handle_route_outcome(event_loop, routed);
    }

    fn route_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        let Some(position) = self.runner.input.last_cursor else {
            return;
        };
        let delta = scroll_delta_to_logical(delta, self.runner.window.dpi_scale);
        if self.runner.can_coalesce_gpu_surface_wheel(position, delta) {
            let modifiers = pointer_modifiers_from_winit(self.runner.input.modifiers);
            self.runner
                .queue_gpu_surface_wheel(position, delta, modifiers);
            return;
        }
        let modifiers = pointer_modifiers_from_winit(self.runner.input.modifiers);
        let routed = if self.runner.can_fast_path_gpu_surface_route(position, delta) {
            self.runner
                .core
                .route_scroll_deferred_refresh_with_modifiers(position, delta, modifiers)
        } else {
            self.runner
                .core
                .route_scroll_with_modifiers(position, delta, modifiers)
        };
        self.runner
            .handle_gpu_surface_route_outcome(routed, position, delta);
    }

    fn take_messages(&mut self) -> Vec<Message> {
        self.runner.core.runtime.bridge_mut().take_messages()
    }
}

pub(super) struct AuxiliaryWindowEventResult<Message> {
    pub(super) closed: bool,
    pub(super) messages: Vec<Message>,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn dispatch_auxiliary_messages(
        &mut self,
        event_loop: &ActiveEventLoop,
        messages: Vec<Message>,
    ) {
        let mut outcome = GenericRouteOutcome::default();
        for message in messages {
            let command_outcome = self.core.runtime.dispatch_message(message);
            outcome.merge(self.core.route_command_outcome(command_outcome));
        }
        self.handle_route_outcome(event_loop, outcome);
        self.sync_auxiliary_windows(event_loop);
    }

    pub(super) fn sync_auxiliary_windows(&mut self, event_loop: &ActiveEventLoop) {
        let projections = self.core.runtime.bridge_mut().project_auxiliary_windows();
        let mut projected_keys = Vec::with_capacity(projections.len());
        for projection in projections {
            projected_keys.push(projection.key.clone());
            if let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| window.key() == projection.key)
            {
                window.update_projection(projection);
            } else {
                let parent_window = self.window.window.as_deref();
                let mut window = AuxiliaryNativeWindow::new(projection, &self.options);
                window.initialize_runtime(event_loop, parent_window);
                self.auxiliary_windows.push(window);
            }
        }
        for window in &mut self.auxiliary_windows {
            if !projected_keys.iter().any(|key| key == window.key()) {
                window.hide();
            }
        }
    }
}
