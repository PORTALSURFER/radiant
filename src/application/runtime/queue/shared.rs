use super::super::subscription::{WorkerSubscriptionDelivery, WorkerSubscriptionIdentity};
use super::super::threading::BusinessThreadPool;
use super::super::timer::{TimerIdentity, TimerLane, TimerSink, TimerWake, timer_sink};
use super::SharedRuntimeDelivery;
use crate::gui::repaint::RepaintSignal;
use crate::runtime::PlatformResultDelivery;
use crate::runtime::{
    RuntimeDiagnostics, RuntimeDiagnosticsRecorder, RuntimeTimerWake, TaskPriority,
};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, OnceLock, Weak,
    atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering},
};
use std::time::Duration;

const SHARED_INGRESS_CAPACITY: usize = 64;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum RuntimeIngressPhase {
    Accepting = 0,
    Closing = 1,
    Stopped = 2,
}

impl RuntimeIngressPhase {
    fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::Accepting,
            1 => Self::Closing,
            _ => Self::Stopped,
        }
    }
}

pub(super) struct Sequenced<T> {
    pub(super) sequence: u64,
    pub(super) value: T,
}

#[derive(Default)]
struct RuntimeAdmission {
    next_sequence: u64,
    deliveries: Vec<Sequenced<SharedRuntimeDelivery>>,
    reservations: usize,
    timer_reservations: HashMap<TimerIdentity, usize>,
}

/// A reserved shared-ingress slot held by an accepted terminal operation.
///
/// Dropping an uncommitted reservation returns the slot without waiting. This
/// lets platform and one-shot timer admission roll back cleanly on shutdown
/// or worker startup failure.
pub(in crate::application::runtime) struct DeliveryReservation {
    runtime: Weak<SharedRuntimeIngress>,
    committed: bool,
}

impl DeliveryReservation {
    fn commit(mut self, delivery: SharedRuntimeDelivery) -> bool {
        let Some(runtime) = self.runtime.upgrade() else {
            self.committed = true;
            return false;
        };
        let accepted = runtime.enqueue_reserved_delivery(delivery);
        if accepted {
            self.committed = true;
        }
        accepted
    }
}

impl Drop for DeliveryReservation {
    fn drop(&mut self) {
        if !self.committed
            && let Some(runtime) = self.runtime.upgrade()
        {
            runtime.release_reservation();
        }
    }
}

pub(in crate::application) struct SharedRuntimeIngress {
    capacity: usize,
    admission: Mutex<RuntimeAdmission>,
    repaint: Mutex<Option<Arc<dyn RepaintSignal>>>,
    business: BusinessThreadPool,
    diagnostics: Arc<RuntimeDiagnosticsRecorder>,
    timers: OnceLock<TimerLane>,
    timer_identities: Mutex<HashMap<TimerIdentity, TimerIdentity>>,
    next_timer_id: AtomicU64,
    timer_epoch: AtomicU64,
    pending_messages: AtomicUsize,
    phase: AtomicU8,
}

impl Default for SharedRuntimeIngress {
    fn default() -> Self {
        let diagnostics = Arc::new(RuntimeDiagnosticsRecorder::default());
        Self {
            capacity: SHARED_INGRESS_CAPACITY,
            admission: Mutex::new(RuntimeAdmission {
                next_sequence: 1,
                deliveries: Vec::new(),
                reservations: 0,
                timer_reservations: HashMap::new(),
            }),
            repaint: Mutex::new(None),
            business: BusinessThreadPool::new_with_diagnostics(Arc::clone(&diagnostics)),
            diagnostics,
            timers: OnceLock::new(),
            timer_identities: Mutex::new(HashMap::new()),
            next_timer_id: AtomicU64::new(1),
            timer_epoch: AtomicU64::new(1),
            pending_messages: AtomicUsize::new(0),
            phase: AtomicU8::new(RuntimeIngressPhase::Accepting as u8),
        }
    }
}

impl SharedRuntimeIngress {
    #[cfg(test)]
    pub(super) fn with_capacity_for_test(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(super) fn reserve_ui_message(&self) -> Option<u64> {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() {
            return None;
        }
        let sequence = next_sequence(&mut admission);
        self.record_message_added();
        Some(sequence)
    }

    pub(in crate::application::runtime) fn reserve_delivery(
        self: &Arc<Self>,
    ) -> Option<DeliveryReservation> {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting()
            || admission
                .deliveries
                .len()
                .saturating_add(admission.reservations)
                >= self.capacity
        {
            self.record_shared_ingress_rejected();
            return None;
        }
        admission.reservations += 1;
        Some(DeliveryReservation {
            runtime: Arc::downgrade(self),
            committed: false,
        })
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
        if !self.is_accepting() {
            return false;
        }
        before_pending_lock();
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() {
            return false;
        }
        if admission
            .deliveries
            .len()
            .saturating_add(admission.reservations)
            >= self.capacity
        {
            self.record_shared_ingress_rejected();
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

    pub(in crate::application::runtime) fn enqueue_worker_disconnect_reserved(
        &self,
        reservation: DeliveryReservation,
        identity: WorkerSubscriptionIdentity,
    ) -> bool {
        reservation.commit(SharedRuntimeDelivery::Worker(
            WorkerSubscriptionDelivery::Disconnected { identity },
        ))
    }

    pub(in crate::application::runtime) fn enqueue_platform_completion_reserved(
        &self,
        reservation: DeliveryReservation,
        delivery: PlatformResultDelivery,
    ) -> bool {
        reservation.commit(SharedRuntimeDelivery::Platform(delivery))
    }

    fn enqueue_reserved_delivery(&self, value: SharedRuntimeDelivery) -> bool {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() || admission.reservations == 0 {
            return false;
        }
        admission.reservations -= 1;
        let sequence = next_sequence(&mut admission);
        admission.deliveries.push(Sequenced { sequence, value });
        self.record_message_added();
        drop(admission);
        self.request_repaint();
        true
    }

    fn release_reservation(&self) {
        let mut admission = lock_runtime_state(&self.admission);
        admission.reservations = admission.reservations.saturating_sub(1);
    }

    pub(super) fn drain_incoming(&self) -> Vec<Sequenced<SharedRuntimeDelivery>> {
        let mut admission = lock_runtime_state(&self.admission);
        drain_runtime_vec(&mut admission.deliveries)
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
        let admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() {
            return false;
        }
        let accepted = self.business.spawn(name, priority, is_cancelled, work);
        drop(admission);
        accepted
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
        let admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() {
            return Err(payload);
        }
        let result = self
            .business
            .spawn_with_payload(name, priority, payload, work);
        drop(admission);
        result
    }

    pub(super) fn can_spawn_business_tasks(&self, priority: TaskPriority) -> bool {
        let _admission = lock_runtime_state(&self.admission);
        self.is_accepting() && self.business.is_available(priority)
    }

    pub(super) fn diagnostics_snapshot(&self) -> RuntimeDiagnostics {
        self.diagnostics.snapshot()
    }

    pub(super) fn install_repaint(&self, signal: Arc<dyn RepaintSignal>) {
        *lock_runtime_state(&self.repaint) = Some(signal);
    }

    pub(in crate::application::runtime) fn request_repaint(&self) {
        if !self.is_accepting() {
            return;
        }
        let signal = {
            let _admission = lock_runtime_state(&self.admission);
            if !self.is_accepting() {
                return;
            }
            lock_runtime_state(&self.repaint).as_ref().map(Arc::clone)
        };
        if let Some(signal) = signal {
            signal.request_repaint();
        }
    }

    pub(in crate::application::runtime) fn begin_closing(&self) -> bool {
        let _admission = lock_runtime_state(&self.admission);
        self.phase
            .compare_exchange(
                RuntimeIngressPhase::Accepting as u8,
                RuntimeIngressPhase::Closing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub(in crate::application::runtime) fn stop(&self) {
        let mut admission = lock_runtime_state(&self.admission);
        if RuntimeIngressPhase::from_raw(self.phase.load(Ordering::Acquire))
            == RuntimeIngressPhase::Accepting
        {
            let _ = self.phase.compare_exchange(
                RuntimeIngressPhase::Accepting as u8,
                RuntimeIngressPhase::Closing as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        if RuntimeIngressPhase::from_raw(self.phase.load(Ordering::Acquire))
            != RuntimeIngressPhase::Closing
        {
            return;
        }
        admission.deliveries.clear();
        admission.reservations = 0;
        admission.timer_reservations.clear();
        self.pending_messages.store(0, Ordering::Release);
        self.diagnostics.record_message_queue_depth(0, 0);
        self.timer_epoch.fetch_add(1, Ordering::AcqRel);
        lock_runtime_state(&self.timer_identities).clear();
        if let Some(timers) = self.timers.get() {
            timers.close();
        }
        self.phase
            .store(RuntimeIngressPhase::Stopped as u8, Ordering::Release);
    }

    #[cfg(test)]
    pub(in crate::application::runtime) fn shutdown(&self) {
        self.begin_closing();
        self.stop();
    }

    pub(in crate::application::runtime) fn is_alive(&self) -> bool {
        let _admission = lock_runtime_state(&self.admission);
        self.is_accepting()
    }

    fn is_accepting(&self) -> bool {
        RuntimeIngressPhase::from_raw(self.phase.load(Ordering::Acquire))
            == RuntimeIngressPhase::Accepting
    }

    pub(super) fn enqueue_frame<Message>(
        &self,
        pending_frame: &mut Option<Message>,
        message: Message,
    ) -> bool {
        let admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() || pending_frame.is_some() {
            return false;
        }
        *pending_frame = Some(message);
        self.record_message_added();
        drop(admission);
        self.request_repaint();
        true
    }

    #[cfg(test)]
    pub(super) fn application_timer_identity_count(&self) -> usize {
        lock_runtime_state(&self.timer_identities).len()
    }

    #[cfg(test)]
    pub(super) fn invalidate_timer_for_test(&self, identity: TimerIdentity) {
        lock_runtime_state(&self.timer_identities).remove(&identity);
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
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() {
            return false;
        }
        if !recurring {
            if admission
                .deliveries
                .len()
                .saturating_add(admission.reservations)
                >= self.capacity
            {
                self.record_shared_ingress_rejected();
                return false;
            }
            admission.reservations += 1;
            *admission.timer_reservations.entry(identity).or_default() += 1;
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
        if !accepted && !recurring {
            release_timer_slot_locked(&mut admission, identity);
        }
        drop(admission);
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

    fn record_shared_ingress_rejected(&self) {
        self.diagnostics.record_shared_ingress_rejected();
    }
}

impl TimerSink for SharedRuntimeIngress {
    fn admit_timer(&self, identity: TimerIdentity) -> bool {
        let _admission = lock_runtime_state(&self.admission);
        self.is_accepting() && self.timer_is_current_locked(identity)
    }

    fn enqueue_timer_wake(&self, wake: TimerWake) -> bool {
        let mut admission = lock_runtime_state(&self.admission);
        if !self.is_accepting() || !self.timer_is_current_locked(wake) {
            release_timer_slot_locked(&mut admission, wake);
            return false;
        }
        if admission.deliveries.iter().any(|delivery| {
            matches!(&delivery.value, SharedRuntimeDelivery::Timer(identity) if *identity == wake)
        }) {
            release_timer_slot_locked(&mut admission, wake);
            self.diagnostics.record_shared_ingress_coalesced();
            drop(admission);
            return false;
        }
        release_timer_slot_locked(&mut admission, wake);
        if admission
            .deliveries
            .len()
            .saturating_add(admission.reservations)
            >= self.capacity
        {
            self.record_shared_ingress_rejected();
            drop(admission);
            return false;
        }
        let sequence = next_sequence(&mut admission);
        admission.deliveries.push(Sequenced {
            sequence,
            value: SharedRuntimeDelivery::Timer(wake),
        });
        self.record_message_added();
        drop(admission);
        self.request_repaint();
        true
    }

    fn timer_is_current(&self, identity: TimerIdentity) -> bool {
        let _admission = lock_runtime_state(&self.admission);
        self.is_accepting() && self.timer_is_current_locked(identity)
    }
}

impl SharedRuntimeIngress {
    fn timer_is_current_locked(&self, identity: TimerIdentity) -> bool {
        identity.owner == crate::runtime::RuntimeTimerOwner::Controller
            || (self.timer_epoch.load(Ordering::Acquire) == identity.epoch
                && lock_runtime_state(&self.timer_identities)
                    .get(&identity)
                    .is_some_and(|current| *current == identity))
    }
}

fn next_sequence(admission: &mut RuntimeAdmission) -> u64 {
    let sequence = admission.next_sequence;
    admission.next_sequence = admission.next_sequence.wrapping_add(1).max(1);
    sequence
}

fn release_timer_slot_locked(admission: &mut RuntimeAdmission, identity: TimerIdentity) {
    if let Some(count) = admission.timer_reservations.get_mut(&identity) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            admission.timer_reservations.remove(&identity);
        }
        admission.reservations = admission.reservations.saturating_sub(1);
    }
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
