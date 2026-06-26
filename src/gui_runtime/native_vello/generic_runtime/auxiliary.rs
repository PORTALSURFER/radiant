use super::{GenericNativeVelloRunner, GenericRouteOutcome, initial_viewport, owner_window_handle};
use crate::runtime::{AuxiliaryWindow, NativeRunOptions, RuntimeBridge};
use bridge::AuxiliarySurfaceBridge;
use placement::centered_position;
use winit::{
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

mod bridge;
mod placement;

pub(super) struct AuxiliaryNativeWindow<Message> {
    key: String,
    close_message: Option<Message>,
    cache_on_close: bool,
    runner: GenericNativeVelloRunner<AuxiliarySurfaceBridge<Message>, Message>,
    active: bool,
}

impl<Message> AuxiliaryNativeWindow<Message> {
    pub(super) fn new(
        projection: AuxiliaryWindow<Message>,
        parent_options: &NativeRunOptions,
    ) -> Self {
        let viewport = initial_viewport(&projection.options);
        let cache_on_close = projection.caches_on_close();
        let mut options = projection.options;
        options.frame.debug_layout |= parent_options.frame.debug_layout;
        if options.text.embedded_fonts.is_empty() && options.text.font_paths.is_empty() {
            options.text = parent_options.text.clone();
        }
        let bridge = AuxiliarySurfaceBridge::new(projection.surface);
        Self {
            key: projection.key,
            close_message: projection.close_message,
            cache_on_close,
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
        self.cache_on_close = projection.caches_on_close();
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
                if self.cache_on_close {
                    self.hide();
                    return AuxiliaryWindowEventResult {
                        closed: false,
                        messages: self.close_message.take().into_iter().collect(),
                    };
                }
                return AuxiliaryWindowEventResult {
                    closed: true,
                    messages: self.close_message.take().into_iter().collect(),
                };
            }
            WindowEvent::Resized(size) => self.runner.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runner.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::CursorEntered { .. } => self.runner.handle_cursor_entered(),
            WindowEvent::CursorMoved { position, .. } => self.runner.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.runner.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                let route = self.runner.route_native_mouse_input(button, state);
                self.runner.handle_route_outcome(event_loop, route.outcome);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let route = self.runner.route_native_mouse_wheel(delta);
                self.runner.handle_route_outcome(event_loop, route.outcome);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.runner.handle_keyboard_event(event_loop, event)
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let routed = self
                    .runner
                    .route_native_modifiers_changed(modifiers.state());
                self.runner.handle_route_outcome(event_loop, routed);
            }
            WindowEvent::RedrawRequested => self.runner.redraw(event_loop),
            _ => {}
        }
        AuxiliaryWindowEventResult {
            closed: false,
            messages: self.take_messages(),
        }
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
        self.timing.deferred_auxiliary_window_sync = false;
        let projections = self.core.runtime.bridge_mut().project_auxiliary_windows();
        for window in &mut self.auxiliary_windows {
            if !auxiliary_projection_contains_key(&projections, window.key()) {
                window.hide();
            }
        }
        for projection in projections {
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
    }

    pub(super) fn defer_auxiliary_window_sync(&mut self) {
        self.timing.deferred_auxiliary_window_sync = true;
    }

    pub(super) fn sync_deferred_auxiliary_windows_if_needed(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) {
        if self.timing.deferred_auxiliary_window_sync {
            self.sync_auxiliary_windows(event_loop);
        }
    }
}

fn auxiliary_projection_contains_key<Message>(
    projections: &[AuxiliaryWindow<Message>],
    key: &str,
) -> bool {
    projections
        .iter()
        .any(|projection| projection.key.as_str() == key)
}

#[cfg(test)]
mod tests {
    use super::auxiliary_projection_contains_key;
    use crate::{application::empty, prelude::IntoView, runtime::AuxiliaryWindow};
    use std::sync::Arc;

    #[test]
    fn auxiliary_projection_key_lookup_uses_projected_windows_without_key_clones() {
        let surface = Arc::new(empty::<()>().into_surface());
        let projections = vec![
            AuxiliaryWindow::new(
                "settings",
                crate::gui_runtime::NativeRunOptions::default(),
                Arc::clone(&surface),
            ),
            AuxiliaryWindow::new(
                "inspector",
                crate::gui_runtime::NativeRunOptions::default(),
                surface,
            ),
        ];

        assert!(auxiliary_projection_contains_key(&projections, "settings"));
        assert!(auxiliary_projection_contains_key(&projections, "inspector"));
        assert!(!auxiliary_projection_contains_key(&projections, "mixer"));
    }
}
