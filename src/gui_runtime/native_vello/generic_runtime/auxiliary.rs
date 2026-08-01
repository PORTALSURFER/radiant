use super::{
    FrameWork, FrameWorkReason, GenericNativeAdapterOwner, GenericNativeVelloRunner,
    GenericRouteOutcome, NativeGenericRunError, NativeResourceMaintenanceTurn, RuntimeUserEvent,
    SceneRebuildMode, initial_viewport, owner_window_handle,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuxiliaryNativeWindowLifecycle {
    Admitted,
    Retiring,
}

pub(super) struct AuxiliaryNativeWindow<Message> {
    key: String,
    close_message: Option<Message>,
    cache_on_close: bool,
    runner: GenericNativeVelloRunner<AuxiliarySurfaceBridge<Message>, Message>,
    active: bool,
    lifecycle: AuxiliaryNativeWindowLifecycle,
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
            lifecycle: AuxiliaryNativeWindowLifecycle::Admitted,
        }
    }

    pub(super) fn key(&self) -> &str {
        &self.key
    }

    fn is_admitted(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Admitted)
    }

    pub(super) fn is_retiring(&self) -> bool {
        matches!(self.lifecycle, AuxiliaryNativeWindowLifecycle::Retiring)
    }

    pub(super) fn maintain_native_resources_with_turn(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        if self.is_retiring() {
            return self.runner.retire_native_resources_with_turn(turn);
        }
        self.runner.maintain_native_resources_with_turn(turn);
        false
    }

    pub(super) fn window_id(&self) -> Option<WindowId> {
        self.is_admitted()
            .then_some(self.runner.window.id)
            .flatten()
    }

    pub(super) fn update_projection(&mut self, projection: AuxiliaryWindow<Message>) {
        if !self.is_admitted() {
            return;
        }
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
        if !self.is_admitted() {
            return;
        }
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
        if !self.is_admitted() {
            return;
        }
        self.runner.queue_accessibility_display_snapshot(snapshot);
    }

    fn begin_retiring(&mut self) {
        if !self.is_admitted() {
            return;
        }
        self.lifecycle = AuxiliaryNativeWindowLifecycle::Retiring;
        self.hide();
        let _ = self.runner.core.runtime.begin_closing();
    }

    fn handle_close_requested(&mut self) -> AuxiliaryWindowEventResult<Message> {
        if self.is_retiring() {
            return AuxiliaryWindowEventResult::ignored();
        }
        if self.cache_on_close {
            self.hide();
            return AuxiliaryWindowEventResult {
                messages: self.close_message.take().into_iter().collect(),
                terminal_cause: None,
            };
        }
        self.begin_retiring();
        AuxiliaryWindowEventResult {
            messages: self.close_message.take().into_iter().collect(),
            terminal_cause: None,
        }
    }

    pub(super) fn route_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> AuxiliaryWindowEventResult<Message> {
        if self.is_retiring() {
            return AuxiliaryWindowEventResult::ignored();
        }
        let mut terminal_cause = None;
        match event {
            WindowEvent::CloseRequested => return self.handle_close_requested(),
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
    pub(super) messages: Vec<Message>,
    pub(super) terminal_cause: Option<NativeGenericRunError>,
}

impl<Message> AuxiliaryWindowEventResult<Message> {
    fn ignored() -> Self {
        Self {
            messages: Vec::new(),
            terminal_cause: None,
        }
    }
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
        let mut maintenance = self.begin_native_resource_maintenance();
        let Some(mut adapter) = self.adapter.take() else {
            return Err(NativeGenericRunError::NativeInitialization {
                stage: super::NativeInitializationStage::DeviceAcquisition,
                message: String::from("native adapter owner was not initialized"),
            });
        };
        let result = self.sync_auxiliary_windows_with_adapter_in_turn(
            event_loop,
            event_proxy,
            &mut adapter,
            &mut maintenance,
        );
        self.adapter = Some(adapter);
        result
    }

    pub(super) fn sync_auxiliary_windows_with_adapter(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
    ) -> Result<(), NativeGenericRunError> {
        let mut maintenance = self.begin_native_resource_maintenance();
        self.sync_auxiliary_windows_with_adapter_in_turn(
            event_loop,
            event_proxy,
            adapter,
            &mut maintenance,
        )
    }

    pub(super) fn sync_auxiliary_windows_with_adapter_in_turn(
        &mut self,
        event_loop: &ActiveEventLoop,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
        adapter: &mut GenericNativeAdapterOwner,
        _maintenance: &mut NativeResourceMaintenanceTurn,
    ) -> Result<(), NativeGenericRunError> {
        self.timing.deferred_auxiliary_window_sync = false;
        if !self.should_admit_auxiliary_sync() {
            return Ok(());
        }
        let projections = self.core.runtime.host_project_auxiliary_windows();
        for window in &mut self.auxiliary_windows {
            if window.is_admitted()
                && !auxiliary_projection_contains_key(&projections, window.key())
            {
                window.hide();
            }
        }
        for projection in projections {
            if let Some(window) = self
                .auxiliary_windows
                .iter_mut()
                .find(|window| window.is_admitted() && window.key() == projection.key)
            {
                window.update_projection(projection);
            } else if auxiliary_key_is_retiring(&self.auxiliary_windows, &projection.key) {
                // Keep the projection pending in application state, but do
                // not reactivate, recreate, or replay it while the older
                // generation-bound child is still retiring.
                continue;
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

fn auxiliary_key_is_retiring<Message>(
    windows: &[AuxiliaryNativeWindow<Message>],
    key: &str,
) -> bool {
    windows
        .iter()
        .any(|window| window.is_retiring() && window.key() == key)
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
        AuxiliaryNativeWindow, AuxiliarySurfaceBridge, AuxiliaryWindowEventResult,
        GenericNativeVelloRunner, NativeResourceMaintenanceTurn,
        append_initialized_auxiliary_window, auxiliary_key_is_retiring,
        auxiliary_projection_contains_key, auxiliary_redraw_terminal_cause,
    };
    use crate::gui::types::Vector2;
    use crate::{
        application::empty, gui_runtime::NativeRunOptions, prelude::IntoView,
        runtime::AuxiliaryWindow,
    };
    use std::sync::Arc;

    fn auxiliary_window(cache_on_close: bool) -> AuxiliaryNativeWindow<i32> {
        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        let projection =
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).on_close(7);
        let projection = if cache_on_close {
            projection.cache_on_close()
        } else {
            projection
        };
        AuxiliaryNativeWindow::new(projection, &NativeRunOptions::default())
    }

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

    #[test]
    fn destructive_close_enters_retiring_and_consumes_its_message_once() {
        let mut window = auxiliary_window(false);

        let first = window.handle_close_requested();
        assert_eq!(first.messages, [7]);
        assert!(window.is_retiring());
        assert!(!window.active);
        assert!(window.window_id().is_none());
        assert!(!window.runner.core.runtime.begin_closing());

        let unrelated = auxiliary_window(true);
        assert!(auxiliary_key_is_retiring(
            std::slice::from_ref(&window),
            "settings"
        ));
        assert!(!auxiliary_key_is_retiring(
            std::slice::from_ref(&unrelated),
            "mixer"
        ));

        let duplicate = window.handle_close_requested();
        assert_eq!(duplicate.messages, Vec::<i32>::new());
        assert!(duplicate.terminal_cause.is_none());

        let late = AuxiliaryWindowEventResult::<i32>::ignored();
        assert!(late.messages.is_empty());
        assert!(late.terminal_cause.is_none());
    }

    #[test]
    fn cached_close_hides_reuses_and_does_not_begin_closing() {
        let mut window = auxiliary_window(true);

        let close = window.handle_close_requested();
        assert_eq!(close.messages, [7]);
        assert!(!window.is_retiring());
        assert!(!window.active);

        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        window.update_projection(
            AuxiliaryWindow::new("settings", NativeRunOptions::default(), surface).cache_on_close(),
        );
        assert!(window.active);
        assert!(!window.is_retiring());
        assert!(window.runner.core.runtime.begin_closing());

        let duplicate = window.handle_close_requested();
        assert!(duplicate.messages.is_empty());
    }

    #[test]
    fn maintenance_removes_retiring_child_only_after_gpu_state_is_empty() {
        let surface = crate::runtime::test_arc_surface(empty::<i32>().into_surface());
        let mut parent = GenericNativeVelloRunner::new(
            NativeRunOptions::default(),
            AuxiliarySurfaceBridge::new(surface),
            Vector2::new(1280.0, 720.0),
        );
        parent.auxiliary_windows.push(auxiliary_window(false));
        let child = parent
            .auxiliary_windows
            .last_mut()
            .expect("test parent should retain the auxiliary child");
        let close = child.handle_close_requested();
        assert_eq!(close.messages, [7]);

        let mut turn = NativeResourceMaintenanceTurn::new();
        assert!(parent.maintain_native_resources_with_turn(&mut turn));

        assert!(parent.auxiliary_windows.is_empty());
        assert!(parent.timing.deferred_auxiliary_window_sync);
        assert!(!turn.has_pending());
    }
}
