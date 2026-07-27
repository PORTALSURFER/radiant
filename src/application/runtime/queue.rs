use super::subscription::WorkerSubscriptionDelivery;
use super::timer::TimerWake;
use crate::gui::repaint::RepaintSignal;
use crate::runtime::{RuntimeDiagnostics, RuntimeTimerWake, TaskPriority};
use std::sync::{Arc, Weak, mpsc};
use std::time::Duration;

mod shared;

use shared::Sequenced;
pub(in crate::application) use shared::SharedRuntimeIngress;

enum PendingMessage<Message> {
    Ordinary(Message),
    Worker(WorkerSubscriptionDelivery),
}

/// UI-owned application runtime state.
///
/// Only [`SharedRuntimeIngress`] is shared with worker and timer lanes. The
/// application message queue and frame slot remain owned by the app bridge.
pub(in crate::application) struct AppRuntime<Message> {
    shared: Arc<SharedRuntimeIngress>,
    pending: Vec<Sequenced<PendingMessage<Message>>>,
    pending_frame: Option<Message>,
    platform_sender: mpsc::Sender<Sequenced<Message>>,
    platform_receiver: mpsc::Receiver<Sequenced<Message>>,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        let (platform_sender, platform_receiver) = mpsc::channel();
        Self {
            shared: Arc::new(SharedRuntimeIngress::default()),
            pending: Vec::new(),
            pending_frame: None,
            platform_sender,
            platform_receiver,
        }
    }
}

impl<Message> AppRuntime<Message> {
    pub(super) fn shared(&self) -> &Arc<SharedRuntimeIngress> {
        &self.shared
    }

    #[cfg(test)]
    pub(super) fn enqueue(&mut self, message: Message) -> bool {
        let Some(sequence) = self.shared.reserve_ui_message() else {
            return false;
        };
        self.pending.push(Sequenced {
            sequence,
            value: PendingMessage::Ordinary(message),
        });
        self.shared.request_repaint();
        true
    }

    pub(super) fn cross_thread_message_sink(&self) -> CrossThreadMessageSink<Message> {
        CrossThreadMessageSink {
            runtime: Arc::downgrade(&self.shared),
            sender: self.platform_sender.clone(),
        }
    }

    pub(super) fn enqueue_frame(&mut self, message: Message) -> bool {
        if !self.shared.is_alive() || self.pending_frame.is_some() {
            return false;
        }
        self.pending_frame = Some(message);
        self.shared.record_frame_added();
        self.shared.request_repaint();
        true
    }

    pub(super) fn spawn_business_task(
        &self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: impl FnOnce() + Send + 'static,
    ) -> bool {
        self.shared
            .spawn_business_task(name, priority, is_cancelled, work)
    }

    pub(super) fn spawn_business_task_with_payload<Payload>(
        &self,
        name: &'static str,
        priority: TaskPriority,
        payload: Payload,
        work: impl FnOnce(Payload) + Send + 'static,
    ) -> Result<(), Payload>
    where
        Payload: Send + 'static,
    {
        self.shared
            .spawn_business_task_with_payload(name, priority, payload, work)
    }

    pub(super) fn can_spawn_business_tasks(&self, priority: TaskPriority) -> bool {
        self.shared.can_spawn_business_tasks(priority)
    }

    pub(super) fn diagnostics_snapshot(&self) -> RuntimeDiagnostics {
        self.shared.diagnostics_snapshot()
    }

    #[cfg(test)]
    pub(super) fn take_pending(&mut self) -> Vec<Message> {
        self.take_pending_with_worker_mapper(|_| None)
    }

    pub(super) fn take_pending_with_worker_mapper(
        &mut self,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> Vec<Message> {
        self.collect_incoming();
        let frame = self.pending_frame.take();
        let drained = self.pending.len() + usize::from(frame.is_some());
        let pending = drain_runtime_vec(&mut self.pending)
            .into_iter()
            .filter_map(|message| message.value.into_message(&mut map_worker))
            .collect();
        self.shared.record_messages_drained(drained);
        prepend_pending_frame(frame, pending)
    }

    #[cfg(test)]
    pub(super) fn drain_pending_into(&mut self, pending: &mut Vec<Message>) {
        self.drain_pending_into_with_worker_mapper(pending, |_| None);
    }

    pub(super) fn drain_pending_into_with_worker_mapper(
        &mut self,
        pending: &mut Vec<Message>,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) {
        self.collect_incoming();
        let mut drained = 0;
        if let Some(frame) = self.pending_frame.take() {
            pending.insert(0, frame);
            drained += 1;
        }
        drained += self.pending.len();
        pending.extend(
            self.pending
                .drain(..)
                .filter_map(|message| message.value.into_message(&mut map_worker)),
        );
        self.shared.record_messages_drained(drained);
    }

    #[cfg(test)]
    pub(super) fn drain_pending_batch_into(
        &mut self,
        pending: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        self.drain_pending_batch_into_with_worker_mapper(pending, max_messages, |_| None)
    }

    pub(super) fn drain_pending_batch_into_with_worker_mapper(
        &mut self,
        pending: &mut Vec<Message>,
        max_messages: usize,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> bool {
        self.collect_incoming();
        let max_messages = max_messages.max(1);
        let mut drained = 0;
        if let Some(frame) = self.pending_frame.take() {
            pending.insert(0, frame);
            drained += 1;
        }
        let available = max_messages.saturating_sub(pending.len());
        if available > 0 {
            let drain_count = self.pending.len().min(available);
            pending.extend(
                self.pending
                    .drain(..drain_count)
                    .filter_map(|message| message.value.into_message(&mut map_worker)),
            );
            drained += drain_count;
        }
        self.shared.record_messages_drained(drained);
        !self.pending.is_empty()
    }

    pub(super) fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        self.shared.install_repaint(signal);
    }

    pub(super) fn request_repaint(&self) {
        self.shared.request_repaint();
    }

    pub(super) fn shutdown(&mut self) {
        self.shared.shutdown();
        self.pending_frame = None;
        self.pending.clear();
        while self.platform_receiver.try_recv().is_ok() {}
    }

    pub(super) fn is_alive(&self) -> bool {
        self.shared.is_alive()
    }

    pub(super) fn take_timer_wakes(&self) -> Vec<TimerWake> {
        self.shared.take_timer_wakes()
    }

    pub(super) fn schedule_timer_wake(&self, delay: Duration, wake: RuntimeTimerWake) -> bool {
        self.shared.schedule_timer_wake(delay, wake)
    }

    fn collect_incoming(&mut self) {
        let (workers, platform) = self
            .shared
            .drain_incoming(|| self.platform_receiver.try_iter().collect::<Vec<_>>());
        self.pending
            .extend(workers.into_iter().map(|delivery| Sequenced {
                sequence: delivery.sequence,
                value: PendingMessage::Worker(delivery.value),
            }));
        self.pending
            .extend(platform.into_iter().map(|message| Sequenced {
                sequence: message.sequence,
                value: PendingMessage::Ordinary(message.value),
            }));
        self.pending.sort_by_key(|message| message.sequence);
    }
}

pub(super) struct CrossThreadMessageSink<Message> {
    runtime: Weak<SharedRuntimeIngress>,
    sender: mpsc::Sender<Sequenced<Message>>,
}

impl<Message> CrossThreadMessageSink<Message> {
    pub(super) fn emit(&self, message: Message) -> bool {
        let Some(runtime) = self.runtime.upgrade() else {
            return false;
        };
        runtime.enqueue_external_message(message, |message| self.sender.send(message).is_ok())
    }
}

impl<Message> PendingMessage<Message> {
    fn into_message(
        self,
        map_worker: &mut impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> Option<Message> {
        match self {
            Self::Ordinary(message) => Some(message),
            Self::Worker(delivery) => map_worker(delivery),
        }
    }
}

fn drain_runtime_vec<T>(queued: &mut Vec<T>) -> Vec<T> {
    let retained_capacity = queued.capacity();
    std::mem::replace(queued, Vec::with_capacity(retained_capacity))
}

fn prepend_pending_frame<T>(frame: Option<T>, mut pending: Vec<T>) -> Vec<T> {
    if let Some(frame) = frame {
        pending.insert(0, frame);
    }
    pending
}

#[cfg(test)]
mod tests;
