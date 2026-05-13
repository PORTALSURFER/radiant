use super::super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    gui::repaint::RepaintSignal,
    runtime::Command,
};
use std::{sync::Arc, time::Duration};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
{
    pub(super) fn install_runtime_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.install_repaint(signal);
        self.run_startup_once();
        self.start_subscriptions_once();
    }

    pub(super) fn schedule_runtime_message(&mut self, delay: Duration, message: Message) -> bool {
        self.runtime.schedule_message(delay, message)
    }

    pub(super) fn spawn_runtime_message_task(
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

    pub(super) fn take_runtime_command_queue(&mut self) -> Vec<Command<Message>> {
        self.runtime.take_commands()
    }

    pub(super) fn drain_runtime_command_queue_into(
        &mut self,
        commands: &mut Vec<Command<Message>>,
    ) {
        self.runtime.drain_commands_into(commands);
    }

    pub(super) fn take_runtime_message_queue(&mut self) -> Vec<Message> {
        self.runtime.take_pending()
    }

    pub(super) fn drain_runtime_message_queue_into(&mut self, messages: &mut Vec<Message>) {
        self.runtime.drain_pending_into(messages);
    }
}
