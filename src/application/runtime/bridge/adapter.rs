use super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    gui::{
        focus::FocusSurface, input::KeyPress, repaint::RepaintSignal, shortcuts::ShortcutResolution,
    },
    runtime::{
        Command, PaintPrimitive, RuntimeAnimationActivity, RuntimeBridge, TransientOverlayContext,
        UiSurface,
    },
};
use std::{sync::Arc, time::Duration};

mod animation;
mod lifecycle;
mod paint;
mod runtime_work;
mod view;

impl<State, Message, Project, Update, View> RuntimeBridge<Message>
    for AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
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

    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<Message>> {
        self.scroll_updated_command(update)
    }

    fn native_file_drop(&mut self, drop: crate::runtime::NativeFileDrop) -> Command<Message> {
        self.native_file_drop_command(drop)
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
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        self.spawn_runtime_message_task(name, work)
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

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime_exit_artifact()
    }

    fn close_requested(&mut self) -> bool {
        self.allow_close_requested()
    }
}
