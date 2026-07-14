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
    Message: Send + 'static,
    State: 'static,
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        self.project_surface_arc()
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        self.pull_surface_owned()
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

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        let capabilities = RuntimeHostCapabilities::new()
            .with_input()
            .with_tasks()
            .with_platform()
            .with_queues()
            .with_animation()
            .with_runtime_diagnostics()
            .with_lifecycle();
        let capabilities = if self.lifecycle.auxiliary_windows.is_some() {
            capabilities.with_windows()
        } else {
            capabilities
        };
        let capabilities = if self.lifecycle.retained_painters.is_empty() {
            capabilities
        } else {
            capabilities.with_retained_surfaces()
        };
        let capabilities = if self.has_app_transient_overlay_painter() {
            capabilities.with_transient_overlays()
        } else {
            capabilities
        };
        if self.lifecycle.native_frame_diagnostics.is_some() {
            capabilities.with_frame_diagnostics()
        } else {
            capabilities
        }
    }
}
