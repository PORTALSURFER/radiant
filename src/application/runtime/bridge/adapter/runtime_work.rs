use super::super::AppBridge;
use crate::{
    application::{IntoView, UiUpdateContext},
    gui::repaint::RepaintSignal,
    runtime::{BusinessMessageSink, Command, TaskPriority},
};
use std::{
    sync::{Arc, Mutex},
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
                let gate = Arc::new(LatestStreamGate::new());
                let emit_runtime = Arc::downgrade(&runtime);
                let emit_latest_runtime = emit_runtime.clone();
                let stale_latest_runtime = emit_runtime.clone();
                let close_gate = Arc::clone(&gate);
                let latest_gate = Arc::clone(&gate);
                drop(runtime);
                let sink = BusinessMessageSink::new_with_latest(
                    move |message| {
                        emit_runtime
                            .upgrade()
                            .is_some_and(|runtime| runtime.enqueue(message))
                    },
                    move |message| {
                        latest_gate.emit_latest(
                            message,
                            |message| {
                                emit_latest_runtime.upgrade().is_some_and(|runtime| {
                                    runtime.enqueue_stream_latest(slot, message)
                                })
                            },
                            || {
                                if let Some(runtime) = stale_latest_runtime.upgrade() {
                                    runtime.record_stale_stream_event();
                                }
                            },
                        )
                    },
                    move || {
                        close_gate.close();
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

struct LatestStreamGate {
    live: Mutex<bool>,
}

impl LatestStreamGate {
    fn new() -> Self {
        Self {
            live: Mutex::new(true),
        }
    }

    fn emit_latest<Message>(
        &self,
        message: Message,
        enqueue: impl FnOnce(Message) -> bool,
        stale: impl FnOnce(),
    ) -> bool {
        let live = self
            .live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !*live {
            drop(live);
            stale();
            return false;
        }
        enqueue(message)
    }

    fn close(&self) {
        *self
            .live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = false;
    }
}

#[cfg(test)]
mod tests {
    use super::LatestStreamGate;
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        thread,
        time::Duration,
    };

    #[test]
    fn latest_stream_gate_rejects_emits_after_close() {
        let gate = LatestStreamGate::new();
        let stale = Arc::new(AtomicBool::new(false));
        let stale_for_emit = Arc::clone(&stale);

        gate.close();
        let accepted = gate.emit_latest(
            1,
            |_| true,
            || {
                stale_for_emit.store(true, Ordering::Release);
            },
        );

        assert!(!accepted);
        assert!(stale.load(Ordering::Acquire));
    }

    #[test]
    fn latest_stream_gate_serializes_close_until_enqueue_finishes() {
        let gate = Arc::new(LatestStreamGate::new());
        let close_gate = Arc::clone(&gate);
        let emit_gate = Arc::clone(&gate);
        let (enqueue_entered_tx, enqueue_entered_rx) = mpsc::channel();
        let (release_enqueue_tx, release_enqueue_rx) = mpsc::channel();
        let (close_attempted_tx, close_attempted_rx) = mpsc::channel();
        let (close_done_tx, close_done_rx) = mpsc::channel();

        let emitter = thread::spawn(move || {
            emit_gate.emit_latest(
                1,
                |_| {
                    enqueue_entered_tx.send(()).expect("enqueue entered");
                    release_enqueue_rx.recv().expect("release enqueue");
                    true
                },
                || {},
            )
        });
        enqueue_entered_rx.recv().expect("enqueue entered");

        let closer = thread::spawn(move || {
            close_attempted_tx.send(()).expect("close attempted");
            close_gate.close();
            close_done_tx.send(()).expect("close done");
        });
        close_attempted_rx.recv().expect("close attempted");
        assert!(
            close_done_rx
                .recv_timeout(Duration::from_millis(20))
                .is_err()
        );

        release_enqueue_tx.send(()).expect("release enqueue");
        assert!(emitter.join().expect("emitter joins"));
        close_done_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("close finishes after enqueue");
        closer.join().expect("closer joins");
    }
}
