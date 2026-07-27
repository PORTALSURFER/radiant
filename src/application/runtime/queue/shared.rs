use super::super::subscription::{WorkerSubscriptionDelivery, WorkerSubscriptionIdentity};
use super::super::threading::BusinessThreadPool;
use super::super::timer::{TimerIdentity, TimerLane, TimerSink, TimerWake, timer_sink};
use super::SharedRuntimeDelivery;
use crate::application::runtime::platform::PlatformCompletionDelivery;
use crate::gui::repaint::RepaintSignal;
use crate::runtime::{
    RuntimeDiagnostics, RuntimeDiagnosticsRecorder, RuntimeTimerWake, TaskPriority,
};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

pub(super) struct Sequenced<T> {
    pub(super) sequence: u64,
    pub(super) value: T,
}

#[derive(Default)]
struct RuntimeAdmission {
    next_sequence: u64,
    deliveries: Vec<Sequenced<SharedRuntimeDelivery>>,
}

pub(in crate::application) struct SharedRuntimeIngress {
    admission: Mutex<RuntimeAdmission>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    timers: OnceLock<TimerLane>,
    timer_wakes: Mutex<Vec<TimerWake>>,
    timer_identities: Mutex<HashMap<TimerIdentity, TimerIdentity>>,
    next_timer_id: AtomicU64,
    timer_epoch: AtomicU64,
    pending_messages: AtomicUsize,
    alive: AtomicBool,
}

impl Default for SharedRuntimeIngress {
    fn default() -> Self {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        Self {
            admission: Mutex::new(RuntimeAdmission {
                next_sequence: 1,
                deliveries: Vec::new(),
            }),
            repaint: Mutex::new(None),
            business: BusinessThreadPool::new_with_diagnostics(Arc::clone(&diagnostics)),
            diagnostics,
            timers: OnceLock::new(),
            timer_wakes: Mutex::new(Vec::new()),
            timer_identities: Mutex::new(HashMap::new()),
            next_timer_id: AtomicU64::new(1),
            timer_epoch: AtomicU64::new(1),
            pending_messages: AtomicUsize::new(0),
            alive: AtomicBool::new(true),
        }
    }
}

impl SharedRuntimeIngress {
    #[cfg(test)]
    pub(super) fn reserve_ui_message(&self) -> Option<u64> {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_alive() {
            return None;
        }
        let sequence = next_sequence(&mut admission);
        self.record_message_added();
        Some(sequence)
    }

    pub(in crate::application::runtime) fn enqueue_worker_payload(
        &self,
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn std::any::Any + Send>,
    ) -> bool {
        self.enqueue_worker_payload_with_pre_append_hook(identity, payload, || {})
    }

    pub(super) fn enqueue_worker_payload_with_pre_append_hook(
        &self,
        identity: WorkerSubscriptionIdentity,
        payload: Box<dyn std::any::Any + Send>,
        before_pending_lock: impl FnOnce(),
    ) -> bool {
        self.enqueue_worker_delivery_with_pre_append_hook(
            WorkerSubscriptionDelivery::Payload { identity, payload },
            before_pending_lock,
        )
    }

    pub(super) fn enqueue_worker_delivery_with_pre_append_hook(
        &self,
        delivery: WorkerSubscriptionDelivery,
        before_pending_lock: impl FnOnce(),
    ) -> bool {
        if !self.is_alive() {
            return false;
        }
        before_pending_lock();
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_alive() {
            return false;
        }
        let sequence = next_sequence(&mut admission);
        admission.deliveries.push(Sequenced {
            sequence,
            value: SharedRuntimeDelivery::Worker(delivery),
        });
        self.record_message_added();
        drop(admission);
        self.request_repaint();
        true
    }

    pub(in crate::application::runtime) fn enqueue_worker_disconnect(
        &self,
        identity: WorkerSubscriptionIdentity,
    ) -> bool {
        self.enqueue_worker_delivery_with_pre_append_hook(
            WorkerSubscriptionDelivery::Disconnected { identity },
            || {},
        )
    }

    pub(in crate::application::runtime) fn enqueue_platform_completion(
        &self,
        delivery: PlatformCompletionDelivery,
    ) -> bool {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_alive() {
            return false;
        }
        let sequence = next_sequence(&mut admission);
        admission.deliveries.push(Sequenced {
            sequence,
            value: SharedRuntimeDelivery::Platform(delivery),
        });
        self.record_message_added();
        drop(admission);
        self.request_repaint();
        true
    }

    pub(super) fn drain_incoming(&self) -> Vec<Sequenced<SharedRuntimeDelivery>> {
        let mut admission = lock_runtime_state(&self.admission);
        drain_runtime_vec(&mut admission.deliveries)
    }

    pub(super) fn record_frame_added(&self) {
        self.record_message_added();
    }

    pub(super) fn record_messages_drained(&self, count: usize) {
        if count == 0 {
            return;
        }
        let mut current = self.pending_messages.load(Ordering::Acquire);
        loop {
            let next = current.saturating_sub(count);
            match self.pending_messages.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.diagnostics.record_message_queue_depth(next, 0);
                    break;
                }
                Err(observed) => current = observed,
            }
        }
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

    pub(super) fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *lock_runtime_state(&self.repaint) = Some(signal);
    }

    pub(in crate::application::runtime) fn request_repaint(&self) {
        let signal = lock_runtime_state(&self.repaint).as_ref().map(Arc::clone);
        if let Some(signal) = signal {
            signal.request_repaint();
        }
    }

    pub(in crate::application::runtime) fn shutdown(&self) {
        let mut admission = lock_runtime_state(&self.admission);
        self.alive.store(false, Ordering::Release);
        admission.deliveries.clear();
        self.pending_messages.store(0, Ordering::Release);
        self.diagnostics.record_message_queue_depth(0, 0);
        drop(admission);

        self.timer_epoch.fetch_add(1, Ordering::AcqRel);
        lock_runtime_state(&self.timer_identities).clear();
        lock_runtime_state(&self.timer_wakes).clear();
        if let Some(timers) = self.timers.get() {
            timers.close();
        }
    }

    pub(in crate::application::runtime) fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    pub(super) fn take_timer_wakes(&self) -> Vec<TimerWake> {
        std::mem::take(&mut *lock_runtime_state(&self.timer_wakes))
    }

    #[cfg(test)]
    pub(super) fn application_timer_identity_count(&self) -> usize {
        lock_runtime_state(&self.timer_identities).len()
    }

    pub(in crate::application::runtime) fn allocate_timer_identity(
        &self,
        generation: u64,
    ) -> TimerIdentity {
        TimerIdentity::application(
            self.next_timer_id.fetch_add(1, Ordering::Relaxed),
            generation,
            self.timer_epoch.load(Ordering::Acquire),
        )
    }

    pub(in crate::application::runtime) fn schedule_timer(
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
        if !accepted && identity.owner == crate::runtime::RuntimeTimerOwner::Application {
            lock_runtime_state(&self.timer_identities).remove(&identity);
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

    fn record_message_added(&self) {
        let depth = self.pending_messages.fetch_add(1, Ordering::AcqRel) + 1;
        self.diagnostics.record_message_queue_depth(depth, 0);
    }
}

impl TimerSink for SharedRuntimeIngress {
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

fn next_sequence(admission: &mut RuntimeAdmission) -> u64 {
    let sequence = admission.next_sequence;
    admission.next_sequence = admission.next_sequence.wrapping_add(1).max(1);
    sequence
}

fn lock_runtime_state<T>(state: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn drain_runtime_vec<T>(queued: &mut Vec<T>) -> Vec<T> {
    let retained_capacity = queued.capacity();
    std::mem::replace(queued, Vec::with_capacity(retained_capacity))
}
