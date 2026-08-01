use super::{
    FrameWork, FrameWorkReason, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    GenericRouteOutcome, NativeGenericRunError, RuntimeUserEvent, SceneRebuildMode,
    initial_viewport, owner_window_handle,
};
use crate::runtime::{AuxiliaryWindow, NativeRunOptions, RuntimeBridge};
use bridge::AuxiliarySurfaceBridge;
use placement::centered_position;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
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
        self.runner
            .request_redraw_for_frame_work(FrameWork::RebuildScene {
                reason: FrameWorkReason::RuntimeSurfaceRefresh,
                mode: SceneRebuildMode::ImmediateWithSurfaceRefresh,
            });
    }

    pub(super) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
        parent_window: Option<&Window>,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
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
        self.runner
            .initialize_runtime_with_adapter(event_loop, event_proxy, adapter)
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

    #[cfg(target_os = "macos")]
    pub(super) fn queue_accessibility_display_snapshot(
        &mut self,
        snapshot: super::window_environment::AccessibilityDisplaySnapshot,
    ) {
        self.runner.queue_accessibility_display_snapshot(snapshot);
    }

    pub(super) fn route_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> AuxiliaryWindowEventResult<Message> {
        let mut terminal_cause = None;
        match event {
            WindowEvent::CloseRequested => {
                if self.cache_on_close {
                    self.hide();
                    return AuxiliaryWindowEventResult {
                        closed: false,
                        messages: self.close_message.take().into_iter().collect(),
                        terminal_cause: None,
                    };
                }
                return AuxiliaryWindowEventResult {
                    closed: true,
                    messages: self.close_message.take().into_iter().collect(),
                    terminal_cause: None,
                };
            }
            WindowEvent::Resized(size) => self.runner.resize_surface(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runner.update_native_dpi_scale(scale_factor);
            }
            WindowEvent::Moved(_) => self.runner.observe_monitor_move(),
            WindowEvent::ThemeChanged(theme) => self.runner.observe_theme_change(Some(theme)),
            WindowEvent::Focused(false) => {
                let routed = self.runner.handle_focus_lost_before_external_drag();
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, routed, adapter);
                if self.runner.core.runtime.external_drag_armed() {
                    let outcome = self.runner.launch_external_drag_if_armed();
                    self.runner
                        .handle_route_outcome_with_adapter(event_loop, outcome, adapter);
                }
            }
            WindowEvent::Focused(true) => {
                let routed = self.runner.handle_focus_regained_after_native_modal_loop();
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, routed, adapter);
            }
            WindowEvent::CursorEntered { .. } => self.runner.handle_cursor_entered(),
            WindowEvent::CursorMoved { position, .. } => self.runner.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => self.runner.handle_cursor_left(event_loop),
            WindowEvent::MouseInput { button, state, .. } => {
                let route = self.runner.route_native_mouse_input(button, state);
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, route.outcome, adapter);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let route = self.runner.route_native_mouse_wheel(delta);
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, route.outcome, adapter);
            }
            WindowEvent::KeyboardInput { event, .. } => self
                .runner
                .handle_keyboard_event_with_adapter(event_loop, event, adapter),
            WindowEvent::ModifiersChanged(modifiers) => {
                let routed = self
                    .runner
                    .route_native_modifiers_changed(modifiers.state());
                self.runner
                    .handle_route_outcome_with_adapter(event_loop, routed, adapter);
            }
            WindowEvent::RedrawRequested => {
                terminal_cause =
                    auxiliary_redraw_terminal_cause(self.runner.redraw(event_loop, adapter));
            }
            _ => {}
        }
        let terminal_cause = terminal_cause.or_else(|| self.runner.take_terminal_cause());
        AuxiliaryWindowEventResult {
            closed: false,
            messages: self.take_messages(),
            terminal_cause,
        }
    }

    fn take_messages(&mut self) -> Vec<Message> {
        self.runner.core.runtime.bridge_mut().take_messages()
    }
}

fn auxiliary_redraw_terminal_cause(
    redraw_result: Result<(), NativeGenericRunError>,
) -> Option<NativeGenericRunError> {
    redraw_result.err()
}

pub(super) struct AuxiliaryWindowEventResult<Message> {
    pub(super) closed: bool,
    pub(super) messages: Vec<Message>,
    pub(super) terminal_cause: Option<NativeGenericRunError>,
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
        if !self.should_admit_auxiliary_sync() {
            return;
        }
        let mut outcome = GenericRouteOutcome::default();
        for message in messages {
            let command_outcome = self.core.runtime.dispatch_message(message);
            outcome.merge(self.core.route_command_outcome(command_outcome));
        }
        self.handle_route_outcome(event_loop, outcome);
        if !self.should_admit_auxiliary_sync() {
            return;
        }
        if let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy() {
            let _ = self.sync_auxiliary_windows(event_loop, event_proxy);
        }
    }

    pub(super) fn sync_auxiliary_windows(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Result<(), NativeGenericRunError> {
        let Some(mut adapter) = self.adapter.take() else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("native adapter owner was not initialized"),
            });
        };
        let result =
            self.sync_auxiliary_windows_with_adapter(event_loop, event_proxy, &mut adapter);
        self.adapter = Some(adapter);
        result
    }

    pub(super) fn sync_auxiliary_windows_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
        self.timing.deferred_auxiliary_window_sync = false;
        if !self.should_admit_auxiliary_sync() {
            return Ok(());
        }
        let projections = self.core.runtime.host_project_auxiliary_windows();
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
                let initialized = {
                    let parent_window = self.window.window.as_deref();
                    let mut window = AuxiliaryNativeWindow::new(projection, &self.options);
                    window
                        .initialize_runtime(event_loop, parent_window, event_proxy.clone(), adapter)
                        .map(|()| window)
                };
                if let Err(error) =
                    append_initialized_auxiliary_window(&mut self.auxiliary_windows, initialized)
                {
                    self.record_initialization_error_and_exit(event_loop, error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    pub(super) fn defer_auxiliary_window_sync(&mut self) {
        self.timing.deferred_auxiliary_window_sync = true;
    }

    pub(super) fn sync_deferred_auxiliary_windows_if_needed(
        &mut self,
        event_loop: &ActiveEventLoop,
        adapter: &mut GenericNativeAdapterOwner,
    ) {
        if self.timing.deferred_auxiliary_window_sync
            && self.should_admit_auxiliary_sync()
            && let Some(event_proxy) = self.runtime_wakeup.event_loop_proxy()
        {
            let _ = self.sync_auxiliary_windows_with_adapter(event_loop, event_proxy, adapter);
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

fn append_initialized_auxiliary_window<T>(
    windows: &mut Vec<T>,
    initialized: Result<T, NativeGenericRunError>,
) -> Result<(), NativeGenericRunError> {
    let window = initialized?;
    windows.push(window);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        append_initialized_auxiliary_window, auxiliary_projection_contains_key,
        auxiliary_redraw_terminal_cause,
    };
    use crate::{application::empty, prelude::IntoView, runtime::AuxiliaryWindow};
    use std::sync::Arc;

    #[test]
    fn auxiliary_projection_key_lookup_uses_projected_windows_without_key_clones() {
        let surface = crate::runtime::test_arc_surface(empty::<()>().into_surface());
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

    #[test]
    fn failed_auxiliary_initialization_propagates_without_appending_child() {
        let failure = crate::gui_runtime::NativeGenericRunError::NativeInitialization {
            stage: crate::gui_runtime::NativeInitializationStage::RendererCreation,
            message: String::from("renderer rejected device"),
        };
        let mut windows = vec![String::from("existing")];

        assert_eq!(
            append_initialized_auxiliary_window(&mut windows, Err(failure.clone())),
            Err(failure)
        );
        assert_eq!(windows, [String::from("existing")]);

        assert_eq!(
            append_initialized_auxiliary_window(&mut windows, Ok(String::from("ready"))),
            Ok(())
        );
        assert_eq!(windows, [String::from("existing"), String::from("ready")]);
    }

    #[test]
    fn auxiliary_redraw_failure_crosses_the_child_event_boundary() {
        let failure = crate::gui_runtime::NativeGenericRunError::FrameRender(String::from(
            "backend rejected scene",
        ));

        assert_eq!(
            auxiliary_redraw_terminal_cause(Err(failure.clone())),
            Some(failure)
        );
        assert_eq!(auxiliary_redraw_terminal_cause(Ok(())), None);
    }
}
