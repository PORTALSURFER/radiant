use super::subscription::WorkerSubscriptionDelivery;
use super::timer::TimerWake;
use crate::gui::repaint::RepaintSignal;
use crate::runtime::PlatformResultDelivery;
use crate::runtime::{
    RuntimeDiagnostics, RuntimeQueueDelivery, RuntimeQueueItem, RuntimeTimerWake, TaskPriority,
};
use std::time::Duration;
use std::{marker::PhantomData, sync::Arc};

mod shared;

use shared::Sequenced;
pub(in crate::application) use shared::SharedRuntimeIngress;

enum PendingMessage<Message> {
    #[cfg(test)]
    Ordinary(Message),
    Shared(SharedRuntimeDelivery, PhantomData<fn() -> Message>),
}

pub(in crate::application::runtime) enum SharedRuntimeDelivery {
    Worker(WorkerSubscriptionDelivery),
    Platform(PlatformResultDelivery),
    Timer(TimerWake),
}

/// UI-owned application runtime state.
///
/// Only [`SharedRuntimeIngress`] is shared with worker and timer lanes. The
/// application message queue and frame slot remain owned by the app bridge.
pub(in crate::application) struct AppRuntime<Message> {
    shared: Arc<SharedRuntimeIngress>,
    pending: Vec<Sequenced<PendingMessage<Message>>>,
    pending_frame: Option<Message>,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        Self {
            shared: Arc::new(SharedRuntimeIngress::default()),
            pending: Vec::new(),
            pending_frame: None,
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
        self.take_pending_with_mappers(|_| None, |_| None, |_| None)
    }

    pub(super) fn take_pending_with_mappers(
        &mut self,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
        mut map_platform: impl FnMut(PlatformResultDelivery) -> Option<Message>,
        mut map_timer: impl FnMut(TimerWake) -> Option<Message>,
    ) -> Vec<Message> {
        self.collect_incoming();
        let frame = self.pending_frame.take();
        let drained = self.pending.len() + usize::from(frame.is_some());
        let pending = drain_runtime_vec(&mut self.pending)
            .into_iter()
            .filter_map(|message| {
                message
                    .value
                    .into_message(&mut map_worker, &mut map_platform, &mut map_timer)
            })
            .collect();
        self.shared.record_messages_drained(drained);
        prepend_pending_frame(frame, pending)
    }

    #[cfg(test)]
    pub(super) fn drain_pending_into(&mut self, pending: &mut Vec<Message>) {
        self.drain_pending_into_with_mappers(pending, |_| None, |_| None, |_| None);
    }

    pub(super) fn drain_pending_into_with_mappers(
        &mut self,
        pending: &mut Vec<Message>,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
        mut map_platform: impl FnMut(PlatformResultDelivery) -> Option<Message>,
        mut map_timer: impl FnMut(TimerWake) -> Option<Message>,
    ) {
        self.collect_incoming();
        let mut drained = 0;
        if let Some(frame) = self.pending_frame.take() {
            pending.insert(0, frame);
            drained += 1;
        }
        drained += self.pending.len();
        pending.extend(self.pending.drain(..).filter_map(|message| {
            message
                .value
                .into_message(&mut map_worker, &mut map_platform, &mut map_timer)
        }));
        self.shared.record_messages_drained(drained);
    }

    #[cfg(test)]
    pub(super) fn drain_pending_batch_into(
        &mut self,
        pending: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        self.drain_pending_batch_into_with_mappers(
            pending,
            max_messages,
            |_| None,
            |_| None,
            |_| None,
        )
    }

    pub(super) fn drain_pending_batch_into_with_mappers(
        &mut self,
        pending: &mut Vec<Message>,
        max_messages: usize,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
        mut map_platform: impl FnMut(PlatformResultDelivery) -> Option<Message>,
        mut map_timer: impl FnMut(TimerWake) -> Option<Message>,
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
            pending.extend(self.pending.drain(..drain_count).filter_map(|message| {
                message
                    .value
                    .into_message(&mut map_worker, &mut map_platform, &mut map_timer)
            }));
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
    }

    pub(super) fn is_alive(&self) -> bool {
        self.shared.is_alive()
    }

    pub(super) fn schedule_timer_wake(&self, delay: Duration, wake: RuntimeTimerWake) -> bool {
        self.shared.schedule_timer_wake(delay, wake)
    }

    fn collect_incoming(&mut self) {
        self.pending.extend(
            self.shared
                .drain_incoming()
                .into_iter()
                .map(|delivery| Sequenced {
                    sequence: delivery.sequence,
                    value: PendingMessage::Shared(delivery.value, PhantomData),
                }),
        );
        self.pending.sort_by_key(|message| message.sequence);
    }

    pub(super) fn drain_pending_item_batch_into(
        &mut self,
        pending: &mut Vec<RuntimeQueueItem<Message>>,
        max_items: usize,
    ) -> bool {
        self.collect_incoming();
        let max_items = max_items.max(1);
        let mut drained = 0;
        if let Some(frame) = self.pending_frame.take() {
            pending.push(RuntimeQueueItem::Message(frame));
            drained += 1;
        }
        let available = max_items.saturating_sub(pending.len());
        if available > 0 {
            let drain_count = self.pending.len().min(available);
            pending.extend(
                self.pending
                    .drain(..drain_count)
                    .map(|message| match message.value {
                        #[cfg(test)]
                        PendingMessage::Ordinary(message) => RuntimeQueueItem::Message(message),
                        PendingMessage::Shared(SharedRuntimeDelivery::Timer(wake), _) => {
                            RuntimeQueueItem::Timer(wake)
                        }
                        PendingMessage::Shared(SharedRuntimeDelivery::Platform(delivery), _) => {
                            RuntimeQueueItem::Delivery(RuntimeQueueDelivery::new(delivery))
                        }
                        PendingMessage::Shared(delivery, _) => {
                            RuntimeQueueItem::Delivery(RuntimeQueueDelivery::new(delivery))
                        }
                    }),
            );
            drained += drain_count;
        }
        self.shared.record_messages_drained(drained);
        !self.pending.is_empty()
    }
}

impl<Message> PendingMessage<Message> {
    fn into_message(
        self,
        map_worker: &mut impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
        map_platform: &mut impl FnMut(PlatformResultDelivery) -> Option<Message>,
        map_timer: &mut impl FnMut(TimerWake) -> Option<Message>,
    ) -> Option<Message> {
        match self {
            #[cfg(test)]
            Self::Ordinary(message) => Some(message),
            Self::Shared(SharedRuntimeDelivery::Worker(delivery), _) => map_worker(delivery),
            Self::Shared(SharedRuntimeDelivery::Platform(delivery), _) => map_platform(delivery),
            Self::Shared(SharedRuntimeDelivery::Timer(wake), _) => map_timer(wake),
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
