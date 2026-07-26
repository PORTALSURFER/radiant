use super::subscription::{WorkerSubscriptionDelivery, WorkerSubscriptionIdentity};
use super::threading::BusinessThreadPool;
use super::timer::{TimerIdentity, TimerLane, TimerSink, TimerWake, timer_sink};
use crate::gui::repaint::RepaintSignal;
use crate::runtime::{
    RuntimeDiagnostics, RuntimeDiagnosticsRecorder, RuntimeTimerWake, TaskPriority,
};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application) struct RuntimeStreamSlot(u64);

enum PendingMessage<Message> {
    Ordinary(Message),
    Worker(WorkerSubscriptionDelivery),
    StreamLatest {
        slot: RuntimeStreamSlot,
        message: Message,
    },
}

pub(in crate::application) struct AppRuntime<Message> {
    pending: Mutex<Vec<PendingMessage<Message>>>,
    pending_frame: Mutex<Option<Message>>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    timers: OnceLock<TimerLane>,
    timer_wakes: Mutex<Vec<TimerWake>>,
    timer_identities: Mutex<HashMap<TimerIdentity, TimerIdentity>>,
    next_timer_id: AtomicU64,
    timer_epoch: AtomicU64,
    alive: AtomicBool,
    next_stream_slot: AtomicU64,
}

impl<Message> Default for AppRuntime<Message> {
    fn default() -> Self {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        Self {
            pending: Mutex::new(Vec::new()),
            pending_frame: Mutex::new(None),
            repaint: Mutex::new(None),
            business: BusinessThreadPool::new_with_diagnostics(Arc::clone(&diagnostics)),
            diagnostics,
            timers: OnceLock::new(),
            timer_wakes: Mutex::new(Vec::new()),
            timer_identities: Mutex::new(HashMap::new()),
            next_timer_id: AtomicU64::new(1),
            timer_epoch: AtomicU64::new(1),
            alive: AtomicBool::new(true),
            next_stream_slot: AtomicU64::new(1),
        }
    }
}

impl<Message> AppRuntime<Message> {
    pub(super) fn enqueue(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        {
            let mut pending = lock_runtime_state(&self.pending);
            pending.push(PendingMessage::Ordinary(message));
            self.record_pending_depth(&pending);
        }
        self.request_repaint();
        true
    }

    pub(super) fn enqueue_worker_payload(
        &self,
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn std::any::Any + Send>,
    ) -> bool {
        self.enqueue_worker_payload_with_pre_append_hook(identity, payload, || {})
    }

    fn enqueue_worker_payload_with_pre_append_hook(
        &self,
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn std::any::Any + Send>,
        before_pending_lock: impl FnOnce(),
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        before_pending_lock();
        {
            let mut pending = lock_runtime_state(&self.pending);
            if !self.is_alive() {
                return false;
            }
            pending.push(PendingMessage::Worker(
                WorkerSubscriptionDelivery::Payload { identity, payload },
            ));
            self.record_pending_depth(&pending);
        }
        self.request_repaint();
        true
    }

    pub(super) fn enqueue_worker_disconnect(&self, identity: WorkerSubscriptionIdentity) -> bool {
        if !self.is_alive() {
            return false;
        }
        {
            let mut pending = lock_runtime_state(&self.pending);
            pending.push(PendingMessage::Worker(
                WorkerSubscriptionDelivery::Disconnected { identity },
            ));
            self.record_pending_depth(&pending);
        }
        self.request_repaint();
        true
    }

    pub(super) fn begin_stream_slot(&self) -> RuntimeStreamSlot {
        RuntimeStreamSlot(self.next_stream_slot.fetch_add(1, Ordering::Relaxed))
    }

    pub(super) fn enqueue_stream_latest(&self, slot: RuntimeStreamSlot, message: Message) -> bool {
        if !self.is_alive() {
            self.diagnostics.record_stream_message_dropped();
            return false;
        }
        {
            let mut pending = lock_runtime_state(&self.pending);
            if let Some(existing) = pending.iter_mut().find_map(|pending| match pending {
                PendingMessage::StreamLatest {
                    slot: pending_slot,
                    message,
                } if *pending_slot == slot => Some(message),
                PendingMessage::Ordinary(_)
                | PendingMessage::Worker(_)
                | PendingMessage::StreamLatest { .. } => None,
            }) {
                *existing = message;
                self.diagnostics.record_stream_message_coalesced();
                self.record_pending_depth(&pending);
            } else {
                pending.push(PendingMessage::StreamLatest { slot, message });
                self.record_pending_depth(&pending);
            }
        }
        self.request_repaint();
        true
    }

    pub(super) fn record_stale_stream_event(&self) {
        self.diagnostics.record_stream_message_stale();
    }

    pub(super) fn enqueue_frame(&self, message: Message) -> bool {
        if !self.is_alive() {
            return false;
        }
        {
            let mut pending_frame = lock_runtime_state(&self.pending_frame);
            if pending_frame.is_some() {
                return false;
            }
            *pending_frame = Some(message);
        }
        self.record_current_pending_depth();
        self.request_repaint();
        true
    }

    pub(super) fn spawn_business_task(
        &self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: impl FnOnce() + Send + 'static,
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        self.business.spawn(name, priority, is_cancelled, work)
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
        if !self.is_alive() {
            return Err(payload);
        }
        self.business
            .spawn_with_payload(name, priority, payload, work)
    }

    pub(super) fn can_spawn_business_tasks(&self, priority: TaskPriority) -> bool {
        self.business.is_available(priority)
    }

    pub(super) fn diagnostics_snapshot(&self) -> RuntimeDiagnostics {
        self.diagnostics.snapshot()
    }

    #[cfg(test)]
    pub(super) fn take_pending(&self) -> Vec<Message> {
        self.take_pending_with_worker_mapper(|_| None)
    }

    pub(super) fn take_pending_with_worker_mapper(
        &self,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> Vec<Message> {
        let frame = lock_runtime_state(&self.pending_frame).take();
        let pending = drain_runtime_vec(&self.pending)
            .into_iter()
            .filter_map(|message| message.into_message(&mut map_worker))
            .collect();
        self.record_current_pending_depth();
        prepend_pending_frame(frame, pending)
    }

    #[cfg(test)]
    pub(super) fn drain_pending_into(&self, pending: &mut Vec<Message>) {
        self.drain_pending_into_with_worker_mapper(pending, |_| None);
    }

    pub(super) fn drain_pending_into_with_worker_mapper(
        &self,
        pending: &mut Vec<Message>,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) {
        if let Some(frame) = lock_runtime_state(&self.pending_frame).take() {
            pending.insert(0, frame);
        }
        let mut queued = lock_runtime_state(&self.pending);
        pending.extend(
            queued
                .drain(..)
                .filter_map(|message| message.into_message(&mut map_worker)),
        );
        self.record_pending_depth(&queued);
    }

    #[cfg(test)]
    pub(super) fn drain_pending_batch_into(
        &self,
        pending: &mut Vec<Message>,
        max_messages: usize,
    ) -> bool {
        self.drain_pending_batch_into_with_worker_mapper(pending, max_messages, |_| None)
    }

    pub(super) fn drain_pending_batch_into_with_worker_mapper(
        &self,
        pending: &mut Vec<Message>,
        max_messages: usize,
        mut map_worker: impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> bool {
        let max_messages = max_messages.max(1);
        if let Some(frame) = lock_runtime_state(&self.pending_frame).take() {
            pending.insert(0, frame);
        }
        let available = max_messages.saturating_sub(pending.len());
        let mut queued = lock_runtime_state(&self.pending);
        if available > 0 {
            let drain_count = queued.len().min(available);
            pending.extend(
                queued
                    .drain(..drain_count)
                    .filter_map(|message| message.into_message(&mut map_worker)),
            );
        }
        let remaining = !queued.is_empty();
        self.record_pending_depth(&queued);
        remaining
    }

    pub(super) fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *lock_runtime_state(&self.repaint) = Some(signal);
    }

    pub(super) fn request_repaint(&self) {
        let signal = lock_runtime_state(&self.repaint).as_ref().map(Arc::clone);
        if let Some(signal) = signal {
            signal.request_repaint();
        }
    }

    pub(super) fn shutdown(&self) {
        self.alive.store(false, Ordering::Release);
        self.timer_epoch.fetch_add(1, Ordering::AcqRel);
        lock_runtime_state(&self.timer_identities).clear();
        lock_runtime_state(&self.timer_wakes).clear();
        if let Some(timers) = self.timers.get() {
            timers.close();
        }
        *lock_runtime_state(&self.pending_frame) = None;
        lock_runtime_state(&self.pending).clear();
        self.record_current_pending_depth();
    }

    pub(super) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    pub(super) fn take_timer_wakes(&self) -> Vec<TimerWake> {
        std::mem::take(&mut *lock_runtime_state(&self.timer_wakes))
    }

    fn record_current_pending_depth(&self) {
        let pending = lock_runtime_state(&self.pending);
        self.record_pending_depth(&pending);
    }

    fn record_pending_depth(&self, pending: &[PendingMessage<Message>]) {
        let pending_frame = lock_runtime_state(&self.pending_frame).is_some() as usize;
        self.diagnostics.record_message_queue_depth(
            pending.len() + pending_frame,
            pending.iter().filter(|message| message.is_stream()).count(),
        );
    }
}

impl<Message> AppRuntime<Message>
where
    Message: Send + 'static,
{
    pub(super) fn allocate_timer_identity(&self, generation: u64) -> TimerIdentity {
        TimerIdentity::application(
            self.next_timer_id.fetch_add(1, Ordering::Relaxed),
            generation,
            self.timer_epoch.load(Ordering::Acquire),
        )
    }

    pub(super) fn schedule_timer(
        self: &Arc<Self>,
        delay: Duration,
        identity: TimerIdentity,
        recurring: bool,
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        if identity.owner == crate::runtime::RuntimeTimerOwner::Application {
            lock_runtime_state(&self.timer_identities).insert(identity, identity);
        }
        let sink = timer_sink(self);
        let timers = self.timers.get_or_init(TimerLane::new);
        let accepted = if recurring {
            timers.schedule_interval(sink, delay, identity)
        } else {
            timers.schedule(sink, delay, identity)
        };
        if !accepted {
            if identity.owner == crate::runtime::RuntimeTimerOwner::Application {
                lock_runtime_state(&self.timer_identities).remove(&identity);
            }
        }
        accepted
    }

    pub(super) fn schedule_timer_wake(
        self: &Arc<Self>,
        delay: Duration,
        wake: RuntimeTimerWake,
    ) -> bool {
        self.schedule_timer(delay, wake, false)
    }
}

impl<Message> TimerSink for AppRuntime<Message>
where
    Message: Send + 'static,
{
    fn admit_timer(&self, identity: TimerIdentity) -> bool {
        if !self.is_alive() {
            return false;
        }
        if identity.owner == crate::runtime::RuntimeTimerOwner::Controller {
            return true;
        }
        self.timer_epoch.load(Ordering::Acquire) == identity.epoch
            && lock_runtime_state(&self.timer_identities)
                .get(&identity)
                .is_some_and(|current| *current == identity)
    }

    fn enqueue_timer_wake(&self, wake: TimerWake) -> bool {
        if !self.admit_timer(wake) {
            return false;
        }
        lock_runtime_state(&self.timer_wakes).push(wake);
        self.request_repaint();
        true
    }

    fn timer_is_current(&self, identity: TimerIdentity) -> bool {
        self.admit_timer(identity)
    }
}

fn lock_runtime_state<T>(state: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn drain_runtime_vec<T>(state: &Mutex<Vec<T>>) -> Vec<T> {
    let mut queued = lock_runtime_state(state);
    let retained_capacity = queued.capacity();
    std::mem::replace(&mut *queued, Vec::with_capacity(retained_capacity))
}

impl<Message> PendingMessage<Message> {
    fn into_message(
        self,
        map_worker: &mut impl FnMut(WorkerSubscriptionDelivery) -> Option<Message>,
    ) -> Option<Message> {
        match self {
            Self::Ordinary(message) | Self::StreamLatest { message, .. } => Some(message),
            Self::Worker(delivery) => map_worker(delivery),
        }
    }

    fn is_stream(&self) -> bool {
        matches!(self, Self::StreamLatest { .. })
    }
}

fn prepend_pending_frame<T>(frame: Option<T>, mut pending: Vec<T>) -> Vec<T> {
    if let Some(frame) = frame {
        pending.insert(0, frame);
    }
    pending
}

#[cfg(test)]
mod tests;
