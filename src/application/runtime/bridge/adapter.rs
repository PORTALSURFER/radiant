use super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::{
        focus::FocusSurface, input::KeyPress, repaint::RepaintSignal, shortcuts::ShortcutResolution,
    },
    runtime::{
        Command, NativeFileOpen, PaintPrimitive, RuntimeAnimationActivity, RuntimeBridge,
        RuntimeDiagnostics, TransientOverlayContext, UiSurface,
    },
};
use std::{sync::Arc, time::Duration};

mod animation;
mod launch_animation;
mod lifecycle;
mod paint;
mod platform_services;
mod runtime_work;
mod view;

impl<State, Message, Project, Update, View> RuntimeBridge<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
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

    fn project_auxiliary_windows(&mut self) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        self.project_app_auxiliary_windows()
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

    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<Message>> {
        self.scroll_updated_command(update)
    }

    fn native_file_drop(&mut self, drop: crate::runtime::NativeFileDrop) -> Command<Message> {
        self.native_file_drop_command(drop)
    }

    fn native_file_open(&mut self, open: NativeFileOpen) -> Command<Message> {
        self.native_file_open_command(open)
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.resolve_shortcut(pending_chord, press, focus)
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.install_runtime_repaint_signal(signal);
    }

    fn schedule_message(&mut self, delay: Duration, message: Message) -> bool {
        self.schedule_runtime_message(delay, message)
    }

    fn spawn_message_task(
        &mut self,
        name: &'static str,
        priority: crate::runtime::TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        self.spawn_runtime_message_task(name, priority, is_cancelled, work)
    }

    fn spawn_streaming_message_task(
        &mut self,
        name: &'static str,
        priority: crate::runtime::TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce(crate::runtime::BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        self.spawn_runtime_streaming_message_task(name, priority, is_cancelled, work)
    }

    fn request_platform_service(
        &mut self,
        request: crate::runtime::PlatformRequest,
        on_completed: crate::runtime::PlatformCompletion<Message>,
    ) -> Result<(), crate::runtime::PlatformServiceFallback<Message>> {
        self.request_app_platform_service(request, on_completed)
    }

    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        self.take_runtime_command_queue()
    }

    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        self.drain_runtime_command_queue_into(commands);
    }

    fn take_runtime_messages(&mut self) -> Vec<Message> {
        self.take_runtime_message_queue()
    }

    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        self.drain_runtime_message_queue_into(messages);
    }

    fn needs_animation(&mut self) -> bool {
        self.needs_runtime_animation()
    }

    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.runtime_animation_activity()
    }

    fn queue_animation_frame(&mut self) -> bool {
        self.queue_runtime_animation_frame()
    }

    fn render_retained_surface(
        &mut self,
        descriptor: crate::widgets::RetainedSurfaceDescriptor,
        rect: crate::gui::types::Rect,
        viewport: crate::layout::Vector2,
    ) -> Option<crate::gui::paint::PaintFrame> {
        self.render_app_retained_surface(descriptor, rect, viewport)
    }

    fn has_transient_overlay_painter(&self) -> bool {
        self.has_app_transient_overlay_painter()
    }

    fn paint_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        self.paint_app_transient_overlay(context, primitives);
    }

    fn has_frame_diagnostics_observer(&self) -> bool {
        false
    }

    fn runtime_diagnostics(&self) -> RuntimeDiagnostics {
        self.runtime.diagnostics_snapshot()
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime_exit_artifact()
    }

    fn close_requested(&mut self) -> bool {
        self.allow_close_requested()
    }
}
