use super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    gui::{
        focus::FocusSurface, input::KeyPress, paint::PaintFrame as GuiPaintFrame,
        repaint::RepaintSignal, shortcuts::ShortcutResolution, types::Rect,
    },
    layout::Vector2,
    runtime::{Command, RuntimeBridge, UiSurface},
    widgets::RetainedSurfaceDescriptor,
};
use std::{sync::Arc, time::Duration};

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
        Arc::new((self.project)(&mut self.state).into_surface())
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        (self.project)(&mut self.state).into_surface()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.run_update(message)
    }

    fn scroll_updated(&mut self, update: crate::runtime::ScrollUpdate) -> Option<Command<Message>> {
        let scroll = self.scroll.as_mut()?;
        let mut context = UpdateContext::default();
        scroll(&mut self.state, update, &mut context);
        Some(context.into_command())
    }

    fn resolve_key_press(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.shortcuts
            .as_mut()
            .map(|shortcuts| shortcuts(&mut self.state, pending_chord, press, focus))
            .unwrap_or_else(ShortcutResolution::unhandled)
    }

    fn install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.install_repaint(signal);
        self.run_startup_once();
        self.start_subscriptions_once();
    }

    fn schedule_message(&mut self, delay: Duration, message: Message) -> bool {
        self.runtime.schedule_message(delay, message)
    }

    fn spawn_message_task(
        &mut self,
        name: &'static str,
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        self.runtime.spawn_business_task(name, move || {
            let message = work();
            if let Some(runtime) = runtime.upgrade() {
                let _ = runtime.enqueue(message);
            }
        })
    }

    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        self.runtime.take_commands()
    }

    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        self.runtime.drain_commands_into(commands);
    }

    fn take_runtime_messages(&mut self) -> Vec<Message> {
        self.runtime.take_pending()
    }

    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        self.runtime.drain_pending_into(messages);
    }

    fn needs_animation(&mut self) -> bool {
        self.animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state))
    }

    fn queue_animation_frame(&mut self) -> bool {
        let active = self
            .animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state));
        if active && let Some(frame_message) = self.frame_message.as_mut() {
            return self.runtime.enqueue_frame(frame_message());
        }
        false
    }

    fn render_retained_surface(
        &mut self,
        descriptor: RetainedSurfaceDescriptor,
        rect: Rect,
        viewport: Vector2,
    ) -> Option<GuiPaintFrame> {
        self.retained_painters
            .get_mut(&descriptor.key)
            .and_then(|paint| paint(&mut self.state, descriptor, rect, viewport))
    }

    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        self.runtime.shutdown();
        self.shutdown
            .as_mut()
            .and_then(|shutdown| shutdown(&mut self.state))
    }

    fn close_requested(&mut self) -> bool {
        self.close_requested
            .as_mut()
            .is_none_or(|close_requested| close_requested(&mut self.state))
    }
}
