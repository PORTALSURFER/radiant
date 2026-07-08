use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::repaint::RepaintSignal,
    runtime::{BusinessMessageSink, Command, TaskPriority},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
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
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        self.runtime
            .spawn_business_task(name, priority, is_cancelled, move || {
                let message = work();
                if let Some(runtime) = runtime.upgrade() {
                    let _ = runtime.enqueue(message);
                }
            })
    }

    pub(super) fn spawn_runtime_streaming_message_task(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        self.runtime
            .spawn_business_task(name, priority, is_cancelled, move || {
                let sink_runtime = runtime.clone();
                let sink = BusinessMessageSink::new(move |message| {
                    sink_runtime
                        .upgrade()
                        .is_some_and(|runtime| runtime.enqueue(message))
                });
                work(sink);
            })
    }

    pub(super) fn spawn_runtime_latest_streaming_message_task(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        if !self.runtime.is_alive() {
            return false;
        }
        let runtime = Arc::downgrade(&self.runtime);
        self.runtime
            .spawn_business_task(name, priority, is_cancelled, move || {
                let Some(runtime) = runtime.upgrade() else {
                    return;
                };
                let slot = runtime.begin_stream_slot();
                let live = Arc::new(AtomicBool::new(true));
                let emit_runtime = Arc::downgrade(&runtime);
                let emit_latest_runtime = emit_runtime.clone();
                let close_live = Arc::clone(&live);
                let latest_live = Arc::clone(&live);
                drop(runtime);
                let sink = BusinessMessageSink::new_with_latest(
                    move |message| {
                        emit_runtime
                            .upgrade()
                            .is_some_and(|runtime| runtime.enqueue(message))
                    },
                    move |message| {
                        if !latest_live.load(Ordering::Acquire) {
                            if let Some(runtime) = emit_latest_runtime.upgrade() {
                                runtime.record_stale_stream_event();
                            }
                            return false;
                        }
                        emit_latest_runtime
                            .upgrade()
                            .is_some_and(|runtime| runtime.enqueue_stream_latest(slot, message))
                    },
                    move || {
                        close_live.store(false, Ordering::Release);
                    },
                );
                work(sink);
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

    pub(super) fn drain_runtime_message_queue_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        self.runtime
            .drain_pending_batch_into(messages, max_messages)
    }
}
