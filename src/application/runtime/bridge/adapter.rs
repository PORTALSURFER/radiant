use super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    runtime::{Command, RuntimeBridge, RuntimeHostCapabilities, UiSurface},
};
use std::sync::Arc;

mod animation;
mod capabilities;
mod launch_animation;
mod lifecycle;
mod paint;
mod platform_services;
mod runtime_work;
mod view;

impl<State, Message, Project, Update, View> RuntimeBridge<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    fn application_environment(&mut self) -> Option<crate::application::ApplicationEnvironment> {
        self.application_environment_for_refresh()
    }

    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        self.project_surface_arc()
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        self.pull_surface_owned()
    }

    fn pull_surface_update(
        &mut self,
        request: crate::runtime::SurfaceRefreshRequest,
    ) -> crate::runtime::SurfaceUpdate<Message> {
        self.pull_application_update(request)
    }

    fn surface_update_provider_authority(
        &self,
    ) -> Option<crate::runtime::SurfaceUpdateProviderAuthority> {
        self.projection_producer.current_authority()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.update_message(message)
    }

    fn update_with_runtime(
        &mut self,
        message: Message,
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<Message> {
        self.update_message_with_runtime(message, snapshot)
    }

    fn set_window_environment(&mut self, environment: crate::runtime::WindowEnvironment) {
        *self.window_environment.borrow_mut() = environment;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        // App views can introduce scene-scoped overlays after any state
        // refresh, so this capability must remain stable for the bridge
        // lifetime even when the initial projection has no overlay.
        let mut capabilities = RuntimeHostCapabilities::new()
            .with_input()
            .with_tasks()
            .with_platform_results()
            .with_queues()
            .with_animation()
            .with_runtime_diagnostics()
            .with_lifecycle()
            .with_transient_overlays();
        if self.lifecycle.auxiliary_windows.is_some() {
            capabilities = capabilities.with_windows();
        }
        if !self.lifecycle.retained_painters.is_empty() {
            capabilities = capabilities.with_retained_surfaces();
        }
        if self.lifecycle.native_frame_diagnostics.is_some() {
            capabilities = capabilities.with_frame_diagnostics();
        }
        if self.lifecycle.native_frame_profile.is_some() {
            capabilities = capabilities.with_frame_profile();
        }
        if self.lifecycle.native_frame_gpu_timing.is_some() {
            capabilities = capabilities.with_frame_gpu_timing();
        }
        if self.lifecycle.native_ime_adapter_observation.is_some() {
            capabilities = capabilities.with_native_ime_adapter_observer();
        }
        capabilities
    }
}
