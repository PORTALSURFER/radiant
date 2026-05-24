use super::*;
use crate::runtime::{AuxiliaryWindow, Command, UiSurface};

pub(super) struct AuxiliaryNativeWindow<Message> {
    key: String,
    close_message: Option<Message>,
    runner: GenericNativeVelloRunner<AuxiliarySurfaceBridge<Message>, Message>,
    active: bool,
}

struct AuxiliarySurfaceBridge<Message> {
    surface: Arc<UiSurface<Message>>,
    outbox: Vec<Message>,
}

impl<Message> AuxiliarySurfaceBridge<Message> {
    fn new(surface: Arc<UiSurface<Message>>) -> Self {
        Self {
            surface,
            outbox: Vec::new(),
        }
    }
}

impl<Message> RuntimeBridge<Message> for AuxiliarySurfaceBridge<Message> {
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::clone(&self.surface)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.outbox.push(message);
        Command::none()
    }
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
        self.runner.window_id
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
        if let Some(window) = self.runner.window.as_ref() {
            window.set_visible(false);
        }
    }

    pub(super) fn show(&mut self) {
        self.active = true;
        if let Some(window) = self.runner.window.as_ref() {
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
            WindowEvent::ScaleFactorChanged { .. } => self.runner.request_redraw_if_needed(),
            WindowEvent::CursorMoved { position, .. } => self.runner.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.runner.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                self.route_mouse_input(event_loop, button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => self.route_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                self.runner.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => self.runner.modifiers = modifiers.state(),
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
        let Some(position) = self.runner.last_cursor else {
            return;
        };
        let Some(button) = pointer_button_from_winit(button) else {
            return;
        };
        let modifiers = pointer_modifiers_from_winit(self.runner.modifiers);
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
        let Some(position) = self.runner.last_cursor else {
            return;
        };
        let delta = scroll_delta_to_logical(delta);
        if self.runner.can_coalesce_gpu_surface_wheel(position, delta) {
            self.runner.queue_gpu_surface_wheel(position, delta);
            return;
        }
        let routed = if self.runner.can_fast_path_gpu_surface_route(position, delta) {
            self.runner
                .core
                .route_scroll_deferred_refresh(position, delta)
        } else {
            self.runner.core.route_scroll(position, delta)
        };
        self.runner
            .handle_gpu_surface_route_outcome(routed, position, delta);
    }

    fn take_messages(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.runner.core.runtime.bridge_mut().outbox)
    }
}

fn centered_position(
    parent_window: Option<&Window>,
    options: &NativeRunOptions,
) -> Option<[f32; 2]> {
    let parent = parent_window?;
    let parent_position = parent.outer_position().ok()?;
    let parent_size = parent.outer_size();
    let scale = parent.scale_factor().max(f64::EPSILON);
    let [child_width, child_height] = options.window.geometry.inner_size.unwrap_or([480.0, 360.0]);
    let child_width = (child_width as f64 * scale).round();
    let child_height = (child_height as f64 * scale).round();
    let x = parent_position.x as f64 + ((parent_size.width as f64 - child_width) / 2.0);
    let y = parent_position.y as f64 + ((parent_size.height as f64 - child_height) / 2.0);
    Some([(x / scale) as f32, (y / scale) as f32])
}

pub(super) struct AuxiliaryWindowEventResult<Message> {
    pub(super) closed: bool,
    pub(super) messages: Vec<Message>,
}
