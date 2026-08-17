//! UI-owned worker-effect completion routing.

use super::SurfaceRuntime;
use super::owner::{
    AuxiliaryWindowOwner, CancellationProbe, EffectOrigin, LifecycleDescriptor, RuntimeOwner,
};
use crate::application::runtime::update_context::business::admission::{
    BusinessTaskAdmission, resolve as resolve_admission,
};
use crate::runtime::RuntimeBridge;
use crate::runtime::command::{
    EffectGeneration, EffectId, WorkerEffectMapper, WorkerEffectSink, WorkerEffectWork,
};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    panic::{self, AssertUnwindSafe},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc::{SyncSender, TrySendError},
    },
};

const EFFECT_INGRESS_CAPACITY: usize = 64;

struct EffectTerminal {
    sequence: u64,
    id: EffectId,
    generation: EffectGeneration,
    registration_id: u64,
    epoch: u64,
    result: EffectResult,
    owner: RuntimeOwner,
    origin: EffectOrigin,
}

enum EffectResult {
    Event(Box<dyn Any + Send>),
    LatestEvent,
    Completed(Box<dyn Any + Send>),
    Cancelled,
    Panicked(String),
}

struct EffectIngress {
    owner: RuntimeOwner,
    sender: SyncSender<EffectTerminal>,
    sequence: Arc<Mutex<u64>>,
    finals: Arc<Mutex<VecDeque<EffectTerminal>>>,
    stream_events_coalesced: Arc<AtomicUsize>,
    stream_events_dropped: Arc<AtomicUsize>,
    stream_events_stale: Arc<AtomicUsize>,
}

impl EffectIngress {
    #[cfg(test)]
    fn send(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        epoch: u64,
        result: EffectResult,
    ) -> bool {
        self.send_with_registration(id, generation, 0, epoch, &EffectOrigin::Application, result)
    }

    fn send_with_registration(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        registration_id: u64,
        epoch: u64,
        origin: &EffectOrigin,
        result: EffectResult,
    ) -> bool {
        let mut sequence = self
            .sequence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let terminal = EffectTerminal {
            sequence: *sequence,
            id,
            generation,
            registration_id,
            epoch,
            result,
            owner: self.owner.clone(),
            origin: origin.clone(),
        };
        match self.sender.try_send(terminal) {
            Ok(()) => {
                *sequence = sequence.saturating_add(1);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    fn send_event_with_registration(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        registration_id: u64,
        epoch: u64,
        origin: &EffectOrigin,
        result: EffectResult,
    ) -> bool {
        let accepted =
            self.send_with_registration(id, generation, registration_id, epoch, origin, result);
        if !accepted {
            self.stream_events_dropped.fetch_add(1, Ordering::AcqRel);
        }
        accepted
    }

    fn send_final_with_registration(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        registration_id: u64,
        epoch: u64,
        origin: &EffectOrigin,
        result: EffectResult,
    ) -> bool {
        let mut sequence = self
            .sequence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let terminal = EffectTerminal {
            sequence: *sequence,
            id,
            generation,
            registration_id,
            epoch,
            result,
            owner: self.owner.clone(),
            origin: origin.clone(),
        };
        *sequence = sequence.saturating_add(1);
        self.finals
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(terminal);
        true
    }

    fn record_coalesced(&self) {
        self.stream_events_coalesced.fetch_add(1, Ordering::AcqRel);
    }

    fn record_stale(&self) {
        self.stream_events_stale.fetch_add(1, Ordering::AcqRel);
    }

    fn high_water(&self) -> u64 {
        *self
            .sequence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn take_finals(&self) -> Vec<EffectTerminal> {
        self.finals
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect()
    }

    fn clone_handle(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            owner: self.owner.clone(),
            sequence: Arc::clone(&self.sequence),
            finals: Arc::clone(&self.finals),
            stream_events_coalesced: Arc::clone(&self.stream_events_coalesced),
            stream_events_dropped: Arc::clone(&self.stream_events_dropped),
            stream_events_stale: Arc::clone(&self.stream_events_stale),
        }
    }
}

struct Registered<Message> {
    generation: EffectGeneration,
    registration_id: u64,
    epoch: u64,
    is_cancelled: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    mapper: RegisteredMapper<Message>,
    lifecycle: LifecycleDescriptor,
    origin: EffectOrigin,
}

enum RegisteredMapper<Message> {
    Once(Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>),
    Stream {
        latest: bool,
        latest_state: Option<Arc<LatestStreamState>>,
        map_event: Box<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        map_final: Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>,
    },
}

struct LatestStreamState {
    gate: Mutex<LatestStreamGate>,
    ingress: EffectIngress,
    id: EffectId,
    generation: EffectGeneration,
    registration_id: u64,
    epoch: u64,
    origin: EffectOrigin,
}

struct LatestStreamGate {
    closed: bool,
    marker_enqueued: bool,
    latest: Option<Box<dyn Any + Send>>,
}

impl LatestStreamState {
    fn emit(&self, payload: Box<dyn Any + Send>) -> bool {
        let mut gate = self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if gate.closed {
            self.ingress.record_stale();
            return false;
        }
        gate.latest = Some(payload);
        if gate.marker_enqueued {
            self.ingress.record_coalesced();
            return true;
        }
        gate.marker_enqueued = true;
        if self.ingress.send_event_with_registration(
            self.id,
            self.generation,
            self.registration_id,
            self.epoch,
            &self.origin,
            EffectResult::LatestEvent,
        ) {
            true
        } else {
            gate.marker_enqueued = false;
            gate.latest = None;
            false
        }
    }

    fn close(&self) {
        self.gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .closed = true;
    }

    fn take_latest(&self) -> Option<Box<dyn Any + Send>> {
        let mut gate = self
            .gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        gate.marker_enqueued = false;
        gate.latest.take()
    }
}

pub(super) struct WorkerEffects<Message> {
    owner: RuntimeOwner,
    ingress: EffectIngress,
    receiver: std::sync::mpsc::Receiver<EffectTerminal>,
    deferred: VecDeque<EffectTerminal>,
    registry: HashMap<EffectId, Registered<Message>>,
    pending_registrations: HashMap<u64, EffectOrigin>,
    pending: usize,
    epoch: u64,
    next_registration_id: u64,
    stream_events_stale: usize,
}

impl<Message> Default for WorkerEffects<Message> {
    fn default() -> Self {
        Self::new(RuntimeOwner::new())
    }
}

impl<Message> WorkerEffects<Message> {
    pub(super) fn new(owner: RuntimeOwner) -> Self {
        let (ingress, receiver) = new_ingress(owner.clone());
        Self {
            owner,
            ingress,
            receiver,
            deferred: VecDeque::new(),
            registry: HashMap::new(),
            pending_registrations: HashMap::new(),
            pending: 0,
            epoch: 1,
            next_registration_id: 1,
            stream_events_stale: 0,
        }
    }

    pub(super) fn submit<Bridge>(
        &mut self,
        runtime: &mut SurfaceRuntime<Bridge, Message>,
        effect: crate::runtime::command::WorkerEffect<Message>,
        origin: EffectOrigin,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        if self.pending >= EFFECT_INGRESS_CAPACITY {
            if let Some(transaction) = effect.transaction {
                transaction.reject();
            }
            if let Some(receipt) = effect.admission_receipt.as_ref() {
                resolve_admission(&receipt.0, BusinessTaskAdmission::Rejected);
            }
            return false;
        }
        let transaction = effect.transaction;
        let id = effect.id;
        let generation = effect.generation;
        let registration_id = self.next_registration_id;
        self.next_registration_id = self.next_registration_id.saturating_add(1);
        let epoch = self.epoch;
        let transaction_probe = transaction
            .as_ref()
            .map(crate::application::LatestTaskTransaction::cancellation_probe);
        let token_probe: Option<CancellationProbe> = effect.is_cancelled.map(Arc::from);
        let origin_probe = origin.cancellation_probe();
        let is_cancelled = combine_cancellation_probes(
            combine_cancellation_probes(token_probe, transaction_probe),
            origin_probe,
        );
        let slot = transaction.as_ref().map(|transaction| transaction.slot());
        let lifecycle = LifecycleDescriptor::new(
            self.owner.clone(),
            id.0,
            slot,
            generation.0,
            is_cancelled.clone().map(|probe| probe as CancellationProbe),
        );
        let (mapper, stream_latest) = match effect.mapper {
            WorkerEffectMapper::Once(map) => (RegisteredMapper::Once(map), None),
            WorkerEffectMapper::Stream {
                latest,
                map_event,
                map_final,
            } => (
                RegisteredMapper::Stream {
                    latest,
                    latest_state: None,
                    map_event,
                    map_final,
                },
                Some(latest),
            ),
        };
        let latest_state = stream_latest.map(|_| {
            Arc::new(LatestStreamState {
                gate: Mutex::new(LatestStreamGate {
                    closed: false,
                    marker_enqueued: false,
                    latest: None,
                }),
                ingress: self.ingress.clone_handle(),
                id,
                generation,
                registration_id,
                epoch,
                origin: origin.clone(),
            })
        });
        let mapper = match mapper {
            RegisteredMapper::Once(map) => RegisteredMapper::Once(map),
            RegisteredMapper::Stream {
                latest,
                latest_state: _,
                map_event,
                map_final,
            } => RegisteredMapper::Stream {
                latest,
                latest_state: latest_state.clone(),
                map_event,
                map_final,
            },
        };
        let previous = self.registry.insert(
            id,
            Registered {
                generation,
                registration_id,
                epoch,
                is_cancelled: is_cancelled.clone(),
                mapper,
                lifecycle,
                origin: origin.clone(),
            },
        );
        self.pending_registrations
            .insert(registration_id, origin.clone());
        self.pending += 1;

        let ingress = Arc::new(self.ingress.clone_handle());
        let work = effect.work;
        let final_ingress = Arc::clone(&ingress);
        let stream_sink = stream_latest.map(|latest| {
            if latest {
                if let Some(latest_state) = latest_state.as_ref().cloned() {
                    let ordered = ingress.clone_handle();
                    let event_origin = origin.clone();
                    WorkerEffectSink::new_latest(
                        move |payload| {
                            ordered.send_event_with_registration(
                                id,
                                generation,
                                registration_id,
                                epoch,
                                &event_origin,
                                EffectResult::Event(payload),
                            )
                        },
                        {
                            let latest_state = Arc::clone(&latest_state);
                            move |payload| latest_state.emit(payload)
                        },
                        move || latest_state.close(),
                    )
                } else {
                    let ordered = Arc::clone(&ingress);
                    let event_origin = origin.clone();
                    WorkerEffectSink::new_ordered(move |payload| {
                        ordered.send_event_with_registration(
                            id,
                            generation,
                            registration_id,
                            epoch,
                            &event_origin,
                            EffectResult::Event(payload),
                        )
                    })
                }
            } else {
                let ordered = Arc::clone(&ingress);
                let event_origin = origin.clone();
                WorkerEffectSink::new_ordered(move |payload| {
                    ordered.send_event_with_registration(
                        id,
                        generation,
                        registration_id,
                        epoch,
                        &event_origin,
                        EffectResult::Event(payload),
                    )
                })
            }
        });
        let final_origin = origin.clone();
        let accepted = runtime.host_spawn_worker_task(
            effect.name,
            effect.priority,
            is_cancelled.as_ref().map(|probe| {
                let probe = Arc::clone(probe);
                Box::new(move || probe()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
            }),
            Box::new(move || {
                if is_cancelled.as_ref().is_some_and(|probe| probe()) {
                    if let Some(state) = latest_state.as_ref() {
                        state.close();
                    }
                    let _ = final_ingress.send_final_with_registration(
                        id,
                        generation,
                        registration_id,
                        epoch,
                        &final_origin,
                        EffectResult::Cancelled,
                    );
                    return;
                }
                let result = panic::catch_unwind(AssertUnwindSafe(|| match work {
                    WorkerEffectWork::Once(work) => work(is_cancelled.clone()),
                    WorkerEffectWork::Stream(work) => {
                        let sink = stream_sink.unwrap_or_else(|| {
                            let ordered = Arc::clone(&final_ingress);
                            let event_origin = final_origin.clone();
                            WorkerEffectSink::new_ordered(move |payload| {
                                ordered.send_event_with_registration(
                                    id,
                                    generation,
                                    registration_id,
                                    epoch,
                                    &event_origin,
                                    EffectResult::Event(payload),
                                )
                            })
                        });
                        work(sink)
                    }
                }));
                let terminal = match result {
                    Ok(_output) if is_cancelled.as_ref().is_some_and(|probe| probe()) => {
                        EffectResult::Cancelled
                    }
                    Ok(output) => EffectResult::Completed(output),
                    Err(payload) => EffectResult::Panicked(panic_message(payload)),
                };
                if let Some(state) = latest_state.as_ref() {
                    state.close();
                }
                let _ = final_ingress.send_final_with_registration(
                    id,
                    generation,
                    registration_id,
                    epoch,
                    &final_origin,
                    terminal,
                );
            }),
        );
        if !accepted {
            self.release_pending(registration_id);
            if let Some(previous) = previous {
                self.registry.insert(id, previous);
            } else {
                self.registry.remove(&id);
            }
            if let Some(transaction) = transaction {
                transaction.reject();
            }
            if let Some(receipt) = effect.admission_receipt.as_ref() {
                resolve_admission(&receipt.0, BusinessTaskAdmission::Rejected);
            }
        } else if let Some(transaction) = transaction {
            if let Some(previous) = previous {
                close_registered_mapper(previous.mapper);
            }
            transaction.accept();
            if let Some(receipt) = effect.admission_receipt.as_ref() {
                resolve_admission(&receipt.0, BusinessTaskAdmission::Accepted);
            }
        } else {
            if let Some(previous) = previous {
                close_registered_mapper(previous.mapper);
            }
            if let Some(receipt) = effect.admission_receipt.as_ref() {
                resolve_admission(&receipt.0, BusinessTaskAdmission::Accepted);
            }
        }
        accepted
    }

    #[cfg(test)]
    pub(super) fn drain(&mut self) -> Vec<Message> {
        self.drain_at_high_water(self.ingress.high_water())
            .into_iter()
            .map(|mapped| mapped.message)
            .collect()
    }

    pub(super) fn drain_with_diagnostics_budget_at_high_water(
        &mut self,
        diagnostics: &crate::runtime::RuntimeDiagnosticsRecorder,
        budget: usize,
        high_water: u64,
    ) -> (Vec<MappedEffectMessage<Message>>, bool, bool) {
        let (messages, deferred, later_turn) = self.drain_at_high_water_budget(high_water, budget);
        let coalesced = self
            .ingress
            .stream_events_coalesced
            .swap(0, Ordering::AcqRel);
        let dropped = self.ingress.stream_events_dropped.swap(0, Ordering::AcqRel);
        let stale =
            self.ingress.stream_events_stale.swap(0, Ordering::AcqRel) + self.stream_events_stale;
        self.stream_events_stale = 0;
        for _ in 0..coalesced {
            diagnostics.record_stream_message_coalesced();
        }
        for _ in 0..dropped {
            diagnostics.record_stream_message_dropped();
        }
        for _ in 0..stale {
            diagnostics.record_stream_message_stale();
        }
        (messages, deferred, later_turn)
    }

    #[cfg(test)]
    fn drain_at_high_water(&mut self, high_water: u64) -> Vec<MappedEffectMessage<Message>> {
        self.drain_at_high_water_budget(high_water, usize::MAX).0
    }

    fn drain_at_high_water_budget(
        &mut self,
        high_water: u64,
        budget: usize,
    ) -> (Vec<MappedEffectMessage<Message>>, bool, bool) {
        let mut terminals = Vec::new();
        let mut deferred = VecDeque::new();
        while let Some(terminal) = self.deferred.pop_front() {
            if terminal.sequence < high_water {
                terminals.push(terminal);
            } else {
                deferred.push_back(terminal);
            }
        }
        while let Ok(terminal) = self.receiver.try_recv() {
            if terminal.sequence >= high_water {
                deferred.push_back(terminal);
                continue;
            }
            terminals.push(terminal);
        }
        for terminal in self.ingress.take_finals() {
            if terminal.sequence >= high_water {
                deferred.push_back(terminal);
            } else {
                terminals.push(terminal);
            }
        }
        terminals.sort_by_key(|terminal| terminal.sequence);
        let deferred_for_budget = terminals.len() > budget;
        let retained = terminals.split_off(terminals.len().min(budget));
        deferred.extend(retained);
        let mut deferred = deferred.into_iter().collect::<Vec<_>>();
        deferred.sort_by_key(|terminal| terminal.sequence);
        self.deferred = deferred.into_iter().collect();
        let mut messages: Vec<MappedEffectMessage<Message>> = Vec::new();
        for terminal in terminals {
            self.apply_terminal(terminal, &mut messages);
        }
        let later_turn = self
            .deferred
            .iter()
            .any(|terminal| terminal.sequence >= high_water);
        (messages, deferred_for_budget, later_turn)
    }

    pub(super) fn high_water(&self) -> u64 {
        self.ingress.high_water()
    }

    pub(super) fn retained_completion_count(&self) -> usize {
        self.deferred.len()
    }

    fn release_pending(&mut self, registration_id: u64) -> bool {
        if registration_id == 0 {
            // Direct controller tests construct registrations without the
            // production registration token. Keep their accounting explicit
            // while all submitted effects use a non-zero token.
            self.pending = self.pending.saturating_sub(1);
            return true;
        }
        if self
            .pending_registrations
            .remove(&registration_id)
            .is_some()
        {
            self.pending = self.pending.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub(super) fn retire_origin(&mut self, origin: &EffectOrigin) {
        let current_ids = self
            .registry
            .iter()
            .filter(|(_, registered)| registered.origin.eq(origin))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in current_ids {
            if let Some(registered) = self.registry.remove(&id) {
                close_registered_mapper(registered.mapper);
            }
        }

        let pending_ids = self
            .pending_registrations
            .iter()
            .filter(|(_, registered_origin)| (*registered_origin).eq(origin))
            .map(|(registration_id, _)| *registration_id)
            .collect::<Vec<_>>();
        for registration_id in pending_ids {
            let _ = self.release_pending(registration_id);
        }
    }

    pub(super) fn retire_auxiliary_owner(&mut self, owner: &AuxiliaryWindowOwner) {
        owner.retire();
        let origin = EffectOrigin::Auxiliary(owner.clone());
        self.retire_origin(&origin);
    }

    fn apply_terminal(
        &mut self,
        terminal: EffectTerminal,
        messages: &mut Vec<MappedEffectMessage<Message>>,
    ) {
        if terminal.epoch != self.epoch {
            return;
        }
        if matches!(
            &terminal.result,
            EffectResult::Completed(_) | EffectResult::Cancelled | EffectResult::Panicked(_)
        ) {
            let _ = self.release_pending(terminal.registration_id);
        }
        let current = self.registry.get(&terminal.id).is_some_and(|entry| {
            entry.generation == terminal.generation
                && (entry.registration_id == terminal.registration_id
                    || terminal.registration_id == 0)
                && entry.epoch == terminal.epoch
                && entry.origin == terminal.origin
                && entry.origin.is_live()
                && entry.lifecycle.admits(
                    &self.owner,
                    terminal.id.0,
                    terminal.generation.0,
                    terminal.owner.is_same(&self.owner),
                )
        });
        if !current {
            if matches!(
                terminal.result,
                EffectResult::Event(_) | EffectResult::LatestEvent
            ) {
                self.stream_events_stale += 1;
            }
            return;
        }
        match terminal.result {
            EffectResult::Event(output) => {
                let Some(entry) = self.registry.get_mut(&terminal.id) else {
                    return;
                };
                if entry.is_cancelled.as_ref().is_some_and(|probe| probe()) {
                    return;
                }
                if let RegisteredMapper::Stream { map_event, .. } = &entry.mapper
                    && let Some(message) = map_event(output)
                {
                    messages.push(MappedEffectMessage {
                        message,
                        origin: entry.origin.clone(),
                    });
                }
            }
            EffectResult::LatestEvent => {
                let Some(entry) = self.registry.get_mut(&terminal.id) else {
                    return;
                };
                if entry.is_cancelled.as_ref().is_some_and(|probe| probe()) {
                    return;
                }
                if let RegisteredMapper::Stream {
                    latest_state: Some(state),
                    map_event,
                    ..
                } = &entry.mapper
                    && let Some(output) = state.take_latest()
                    && let Some(message) = map_event(output)
                {
                    messages.push(MappedEffectMessage {
                        message,
                        origin: entry.origin.clone(),
                    });
                }
            }
            EffectResult::Completed(output) => {
                let Some(entry) = self.registry.remove(&terminal.id) else {
                    return;
                };
                if !entry.origin.is_live()
                    || entry.is_cancelled.as_ref().is_some_and(|probe| probe())
                {
                    return;
                }
                let origin = entry.origin.clone();
                match entry.mapper {
                    RegisteredMapper::Once(map) => {
                        if let Some(message) = map(output) {
                            messages.push(MappedEffectMessage { message, origin });
                        }
                    }
                    RegisteredMapper::Stream { map_final, .. } => {
                        if let Some(message) = map_final(output) {
                            messages.push(MappedEffectMessage { message, origin });
                        }
                    }
                }
            }
            EffectResult::Panicked(message) => {
                let Some(entry) = self.registry.remove(&terminal.id) else {
                    return;
                };
                if let RegisteredMapper::Stream {
                    latest_state: Some(state),
                    ..
                } = entry.mapper
                {
                    state.close();
                }
                tracing::error!(effect.id = terminal.id.0, %message, "Radiant worker effect panicked")
            }
            EffectResult::Cancelled => {
                let Some(entry) = self.registry.remove(&terminal.id) else {
                    return;
                };
                if let RegisteredMapper::Stream {
                    latest_state: Some(state),
                    ..
                } = entry.mapper
                {
                    state.close();
                }
            }
        }
    }

    pub(super) fn shutdown(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        self.registry.clear();
        self.pending_registrations.clear();
        self.deferred.clear();
        let (ingress, receiver) = new_ingress(self.owner.clone());
        self.ingress = ingress;
        self.receiver = receiver;
        self.pending = 0;
        self.stream_events_stale = 0;
    }
}

pub(super) struct MappedEffectMessage<Message> {
    pub(super) message: Message,
    pub(super) origin: EffectOrigin,
}

fn close_registered_mapper<Message>(mapper: RegisteredMapper<Message>) {
    if let RegisteredMapper::Stream {
        latest_state: Some(state),
        ..
    } = mapper
    {
        state.close();
    }
}

fn new_ingress(owner: RuntimeOwner) -> (EffectIngress, std::sync::mpsc::Receiver<EffectTerminal>) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(EFFECT_INGRESS_CAPACITY);
    let finals = Arc::new(Mutex::new(VecDeque::new()));
    (
        EffectIngress {
            owner,
            sender,
            sequence: Arc::new(Mutex::new(0)),
            finals,
            stream_events_coalesced: Arc::new(AtomicUsize::new(0)),
            stream_events_dropped: Arc::new(AtomicUsize::new(0)),
            stream_events_stale: Arc::new(AtomicUsize::new(0)),
        },
        receiver,
    )
}

fn combine_cancellation_probes(
    first: Option<CancellationProbe>,
    second: Option<CancellationProbe>,
) -> Option<CancellationProbe> {
    match (first, second) {
        (None, None) => None,
        (Some(probe), None) | (None, Some(probe)) => Some(probe),
        (Some(first), Some(second)) => Some(Arc::new(move || first() || second())),
    }
}

impl<Message> Drop for WorkerEffects<Message> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    payload
        .downcast_ref::<&'static str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload")
        .to_owned()
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn submit_worker_effect_with_origin(
        &mut self,
        effect: crate::runtime::command::WorkerEffect<Message>,
        origin: EffectOrigin,
    ) -> bool {
        let mut effects = std::mem::take(&mut self.worker_effects);
        let accepted = effects.submit(self, effect, origin);
        self.worker_effects = effects;
        accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ContainerPolicy;
    use crate::runtime::command::{EffectGeneration, EffectId};
    use crate::runtime::{
        DragPreview, DragRequest, ExternalDragEffect, ExternalDragOutcome, ExternalDragRequest,
        PlatformResultDelivery, RuntimeDiagnosticsRecorder, RuntimeHostCapabilities,
        RuntimeQueueHost, RuntimeTaskHost, RuntimeTimerWake, SurfaceNode, UiSurface,
    };
    use crate::{
        application::{IntoView, column, text},
        gui::types::{Point, Vector2},
        runtime::SurfaceRuntime,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    fn register(effect: &mut WorkerEffects<usize>, id: u64, generation: u64) {
        effect.registry.insert(
            EffectId(id),
            Registered {
                generation: EffectGeneration(generation),
                registration_id: 0,
                epoch: effect.epoch,
                is_cancelled: None,
                lifecycle: LifecycleDescriptor::new(
                    effect.owner.clone(),
                    id,
                    None,
                    generation,
                    None,
                ),
                mapper: RegisteredMapper::Once(Box::new(|output| {
                    Some(*output.downcast::<usize>().expect("usize output"))
                })),
                origin: EffectOrigin::Application,
            },
        );
        effect.pending += 1;
    }

    fn register_owned(
        effect: &mut WorkerEffects<usize>,
        id: u64,
        generation: u64,
        registration_id: u64,
        origin: EffectOrigin,
    ) {
        effect.registry.insert(
            EffectId(id),
            Registered {
                generation: EffectGeneration(generation),
                registration_id,
                epoch: effect.epoch,
                is_cancelled: None,
                lifecycle: LifecycleDescriptor::new(
                    effect.owner.clone(),
                    id,
                    None,
                    generation,
                    None,
                ),
                mapper: RegisteredMapper::Once(Box::new(|output| {
                    Some(*output.downcast::<usize>().expect("usize output"))
                })),
                origin: origin.clone(),
            },
        );
        effect.pending_registrations.insert(registration_id, origin);
        effect.pending += 1;
    }

    fn register_stream_owned(
        effect: &mut WorkerEffects<usize>,
        id: u64,
        registration_id: u64,
        origin: EffectOrigin,
    ) -> Arc<LatestStreamState> {
        let state = Arc::new(LatestStreamState {
            gate: Mutex::new(LatestStreamGate {
                closed: false,
                marker_enqueued: false,
                latest: None,
            }),
            ingress: effect.ingress.clone_handle(),
            id: EffectId(id),
            generation: EffectGeneration(1),
            registration_id,
            epoch: effect.epoch,
            origin: origin.clone(),
        });
        effect.registry.insert(
            EffectId(id),
            Registered {
                generation: EffectGeneration(1),
                registration_id,
                epoch: effect.epoch,
                is_cancelled: None,
                lifecycle: LifecycleDescriptor::new(effect.owner.clone(), id, None, 1, None),
                mapper: RegisteredMapper::Stream {
                    latest: true,
                    latest_state: Some(Arc::clone(&state)),
                    map_event: Box::new(|output| {
                        Some(*output.downcast::<usize>().expect("usize output"))
                    }),
                    map_final: Box::new(|output| {
                        Some(*output.downcast::<usize>().expect("usize output"))
                    }),
                },
                origin: origin.clone(),
            },
        );
        effect.pending_registrations.insert(registration_id, origin);
        effect.pending += 1;
        state
    }

    fn declarative_origins() -> (EffectOrigin, EffectOrigin, EffectOrigin) {
        let phase = Rc::new(Cell::new(0_u8));
        let project_phase = Rc::clone(&phase);
        let mut runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            move |_| {
                if project_phase.get() == 1 {
                    text::<usize>("raw").into_surface()
                } else {
                    column([text::<usize>("old").key("old")]).into_surface()
                }
            },
            |_, _| {},
        );
        let old = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("old declarative owner")
            .token
            .clone();
        let sibling_runtime = SurfaceRuntime::new_declarative_owned(
            (),
            Vector2::new(80.0, 40.0),
            |_| column([text::<usize>("sibling").key("sibling")]).into_surface(),
            |_, _| {},
        );
        let sibling = sibling_runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("sibling declarative owner")
            .token
            .clone();
        phase.set(1);
        runtime.refresh();
        phase.set(2);
        runtime.refresh();
        let new = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("later declarative owner generation")
            .token
            .clone();
        (
            EffectOrigin::Declarative(old),
            EffectOrigin::Declarative(sibling),
            EffectOrigin::Declarative(new),
        )
    }

    #[test]
    fn completion_arriving_after_turn_snapshot_waits_for_next_turn() {
        let mut effects = WorkerEffects::<usize>::default();
        let high_water = effects.ingress.high_water();
        register(&mut effects, 1, 1);
        assert!(effects.ingress.send(
            EffectId(1),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Completed(Box::new(7_usize)),
        ));
        let (messages, budget_deferred, later_turn) =
            effects.drain_at_high_water_budget(high_water, 64);
        assert!(messages.is_empty());
        assert!(!budget_deferred);
        assert!(
            later_turn,
            "post-snapshot completion must request a later turn"
        );
        assert_eq!(effects.drain(), vec![7]);
    }

    #[test]
    fn post_snapshot_terminal_burst_is_deferred_without_receiver_starvation() {
        let mut effects = WorkerEffects::<usize>::default();
        let diagnostics = RuntimeDiagnosticsRecorder::default();
        let high_water = effects.ingress.high_water();
        for id in 1..=3 {
            register(&mut effects, id, 1);
            assert!(effects.ingress.send(
                EffectId(id),
                EffectGeneration(1),
                effects.epoch,
                EffectResult::Completed(Box::new(id as usize)),
            ));
        }

        let (messages, budget_deferred, later_turn) =
            effects.drain_at_high_water_budget(high_water, 64);
        assert!(messages.is_empty());
        assert!(!budget_deferred);
        assert!(later_turn);
        assert_eq!(effects.retained_completion_count(), 3);
        diagnostics.record_controller_completion_depth(effects.retained_completion_count());
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.queue.current_pending_controller_completions, 3);
        assert_eq!(snapshot.queue.max_pending_controller_completions, 3);
        assert_eq!(effects.drain(), vec![1, 2, 3]);
    }

    #[test]
    fn stale_generation_is_rejected_before_mapper_and_cleans_terminal() {
        let mut effects = WorkerEffects::<usize>::default();
        register(&mut effects, 2, 2);
        assert!(effects.ingress.send(
            EffectId(2),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Completed(Box::new(9_usize)),
        ));
        assert!(effects.drain().is_empty());
        assert_eq!(effects.pending, 0);
        assert!(effects.registry.contains_key(&EffectId(2)));
    }

    #[test]
    fn retiring_auxiliary_owner_keeps_siblings_and_releases_pending_once() {
        let mut effects = WorkerEffects::<usize>::default();
        let owner_a = AuxiliaryWindowOwner::new("window-a");
        let owner_b = AuxiliaryWindowOwner::new("window-b");
        let origin_a = EffectOrigin::Auxiliary(owner_a.clone());
        let origin_b = EffectOrigin::Auxiliary(owner_b.clone());
        register_owned(&mut effects, 41, 1, 101, origin_a.clone());
        register_owned(&mut effects, 42, 1, 102, origin_b.clone());

        effects.retire_auxiliary_owner(&owner_a);

        assert!(!owner_a.is_open());
        assert!(owner_b.is_open());
        assert!(!effects.registry.contains_key(&EffectId(41)));
        assert!(effects.registry.contains_key(&EffectId(42)));
        assert_eq!(effects.pending, 1);

        assert!(effects.ingress.send_with_registration(
            EffectId(41),
            EffectGeneration(1),
            101,
            effects.epoch,
            &origin_a,
            EffectResult::Completed(Box::new(410_usize)),
        ));
        assert!(effects.ingress.send_with_registration(
            EffectId(42),
            EffectGeneration(1),
            102,
            effects.epoch,
            &origin_b,
            EffectResult::Completed(Box::new(420_usize)),
        ));
        assert_eq!(effects.drain(), vec![420]);
        assert_eq!(effects.pending, 0);

        // A duplicate late terminal cannot decrement the already-released
        // registration or disturb the sibling result.
        assert!(effects.ingress.send_with_registration(
            EffectId(41),
            EffectGeneration(1),
            101,
            effects.epoch,
            &origin_a,
            EffectResult::Completed(Box::new(411_usize)),
        ));
        assert!(effects.drain().is_empty());
        assert_eq!(effects.pending, 0);
    }

    #[test]
    fn declarative_retirement_closes_stream_releases_pending_and_isolates_origins() {
        let (old_origin, sibling_origin, new_origin) = declarative_origins();
        assert!(!old_origin.eq(&new_origin));
        assert!(new_origin.is_live());
        let mut effects = WorkerEffects::<usize>::default();
        let stream_state = register_stream_owned(&mut effects, 61, 601, old_origin.clone());
        register_owned(&mut effects, 62, 1, 602, sibling_origin.clone());
        register_owned(&mut effects, 63, 1, 603, EffectOrigin::Application);
        register_owned(&mut effects, 64, 1, 604, new_origin.clone());

        effects.retire_origin(&old_origin);

        assert!(
            stream_state
                .gate
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .closed
        );
        assert!(!effects.registry.contains_key(&EffectId(61)));
        assert!(effects.registry.contains_key(&EffectId(62)));
        assert!(effects.registry.contains_key(&EffectId(63)));
        assert!(effects.registry.contains_key(&EffectId(64)));
        assert_eq!(effects.pending, 3);

        let send = |effects: &WorkerEffects<usize>, id, registration_id, origin| {
            effects.ingress.send_with_registration(
                EffectId(id),
                EffectGeneration(1),
                registration_id,
                effects.epoch,
                origin,
                EffectResult::Completed(Box::new(id as usize)),
            )
        };
        assert!(send(&effects, 61, 601, &old_origin));
        assert!(send(&effects, 62, 602, &sibling_origin));
        assert!(send(&effects, 63, 603, &EffectOrigin::Application));
        assert!(send(&effects, 64, 604, &new_origin));
        assert_eq!(effects.drain(), vec![62, 63, 64]);
        assert_eq!(effects.pending, 0);

        effects.retire_origin(&old_origin);
        assert_eq!(effects.pending, 0);
        assert!(effects.ingress.send_with_registration(
            EffectId(61),
            EffectGeneration(1),
            601,
            effects.epoch,
            &old_origin,
            EffectResult::Completed(Box::new(610_usize)),
        ));
        assert!(effects.drain().is_empty());
    }

    #[test]
    fn same_key_reopen_rejects_old_generation_without_removing_new_replacement() {
        let mut effects = WorkerEffects::<usize>::default();
        let old_owner = AuxiliaryWindowOwner::new("settings");
        let old_origin = EffectOrigin::Auxiliary(old_owner.clone());
        register_owned(&mut effects, 51, 1, 201, old_origin.clone());
        effects.retire_auxiliary_owner(&old_owner);

        let new_owner = AuxiliaryWindowOwner::new("settings");
        assert!(!old_owner.is_same_generation(&new_owner));
        let new_origin = EffectOrigin::Auxiliary(new_owner.clone());
        register_owned(&mut effects, 51, 1, 202, new_origin.clone());

        assert!(effects.ingress.send_with_registration(
            EffectId(51),
            EffectGeneration(1),
            201,
            effects.epoch,
            &old_origin,
            EffectResult::Completed(Box::new(510_usize)),
        ));
        assert!(effects.ingress.send_with_registration(
            EffectId(51),
            EffectGeneration(1),
            202,
            effects.epoch,
            &new_origin,
            EffectResult::Completed(Box::new(520_usize)),
        ));

        assert_eq!(effects.drain(), vec![520]);
        assert_eq!(effects.pending, 0);
    }

    #[test]
    fn auxiliary_owner_registry_retires_exact_generation_and_stays_bounded() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let old_settings = runtime.acquire_auxiliary_effect_owner("settings");
        let inspector = runtime.acquire_auxiliary_effect_owner("inspector");
        assert_eq!(runtime.auxiliary_effect_owners.len(), 2);
        assert!(runtime.auxiliary_effect_owner_is_active(&old_settings));
        assert!(runtime.auxiliary_effect_owner_is_active(&inspector));

        assert!(runtime.retire_auxiliary_effect_owner(&old_settings));
        let new_settings = runtime.acquire_auxiliary_effect_owner("settings");
        assert!(!old_settings.is_same_generation(&new_settings));
        assert!(!runtime.auxiliary_effect_owner_is_active(&old_settings));
        assert!(runtime.auxiliary_effect_owner_is_active(&new_settings));
        assert!(runtime.auxiliary_effect_owner_is_active(&inspector));
        assert_eq!(runtime.auxiliary_effect_owners.len(), 2);

        // A late retirement from the old native child cannot remove the new
        // same-key generation.
        assert!(!runtime.retire_auxiliary_effect_owner(&old_settings));
        assert!(runtime.auxiliary_effect_owner_is_active(&new_settings));
    }

    #[test]
    fn auxiliary_origin_survives_worker_completion_and_chained_command() {
        let mut runtime =
            SurfaceRuntime::new(OriginBridge { show_owner: true }, Vector2::new(80.0, 40.0));
        let owner = runtime.acquire_auxiliary_effect_owner("settings");

        let _ = runtime.dispatch_message_from_auxiliary(1, owner.clone());
        let (first_generation, first_registration, first_epoch, first_origin) = {
            let entry = runtime
                .worker_effects
                .registry
                .get(&EffectId(1))
                .expect("first auxiliary worker registration");
            (
                entry.generation,
                entry.registration_id,
                entry.epoch,
                entry.origin.clone(),
            )
        };
        assert!(matches!(
            &first_origin,
            EffectOrigin::Auxiliary(actual) if actual.is_same_generation(&owner)
        ));
        assert!(runtime.worker_effects.ingress.send_with_registration(
            EffectId(1),
            first_generation,
            first_registration,
            first_epoch,
            &first_origin,
            EffectResult::Completed(Box::new(1_usize)),
        ));

        let _ = runtime.drain_runtime_messages();
        let (second_generation, second_registration, second_epoch, second_origin) = {
            let entry = runtime
                .worker_effects
                .registry
                .get(&EffectId(2))
                .expect("chained auxiliary worker registration");
            (
                entry.generation,
                entry.registration_id,
                entry.epoch,
                entry.origin.clone(),
            )
        };
        assert!(matches!(
            &second_origin,
            EffectOrigin::Auxiliary(actual) if actual.is_same_generation(&owner)
        ));
        assert!(runtime.worker_effects.ingress.send_with_registration(
            EffectId(2),
            second_generation,
            second_registration,
            second_epoch,
            &second_origin,
            EffectResult::Completed(Box::new(2_usize)),
        ));

        let _ = runtime.drain_runtime_messages();
        assert!(runtime.worker_effects.registry.is_empty());
        assert_eq!(runtime.worker_effects.pending, 0);
    }

    #[test]
    fn declarative_origin_survives_worker_chain_and_vetoes_late_completion() {
        let mut runtime =
            SurfaceRuntime::new(OriginBridge { show_owner: true }, Vector2::new(80.0, 40.0));
        let token = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("keyed declarative owner")
            .token
            .clone();
        let origin = EffectOrigin::Declarative(token.clone());

        let mut outcome = crate::runtime::CommandOutcome::default();
        runtime.dispatch_message_inner_with_origin(1, &mut outcome, origin.clone());
        let first = runtime
            .worker_effects
            .registry
            .get(&EffectId(1))
            .expect("first declarative worker registration");
        let first_registration = (first.generation, first.registration_id, first.epoch);
        assert!(first.origin == origin);
        assert!(runtime.worker_effects.ingress.send_with_registration(
            EffectId(1),
            first_registration.0,
            first_registration.1,
            first_registration.2,
            &origin,
            EffectResult::Completed(Box::new(1_usize)),
        ));

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        let second = runtime
            .worker_effects
            .registry
            .get(&EffectId(2))
            .expect("chained declarative worker registration");
        let second_registration = (second.generation, second.registration_id, second.epoch);
        assert!(second.origin == origin);
        assert!(runtime.worker_effects.ingress.send_with_registration(
            EffectId(2),
            second_registration.0,
            second_registration.1,
            second_registration.2,
            &origin,
            EffectResult::Completed(Box::new(2_usize)),
        ));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert!(runtime.worker_effects.registry.is_empty());

        let mut outcome = crate::runtime::CommandOutcome::default();
        runtime.dispatch_message_inner_with_origin(1, &mut outcome, origin.clone());
        let late = runtime
            .worker_effects
            .registry
            .get(&EffectId(1))
            .expect("late declarative worker registration");
        let late_registration = (late.generation, late.registration_id, late.epoch);
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert!(!token.is_live());
        assert!(runtime.worker_effects.ingress.send_with_registration(
            EffectId(1),
            late_registration.0,
            late_registration.1,
            late_registration.2,
            &origin,
            EffectResult::Completed(Box::new(3_usize)),
        ));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn auxiliary_origin_survives_timer_completion_and_chained_command() {
        let mut runtime = SurfaceRuntime::new(
            TimerOriginBridge {
                scheduled: Vec::new(),
                dispatched: Vec::new(),
            },
            Vector2::new(80.0, 40.0),
        );
        let owner = runtime.acquire_auxiliary_effect_owner("settings");

        let _ = runtime.dispatch_message_from_auxiliary(1, owner.clone());
        let first_wake = runtime.bridge().scheduled[0];
        let first_origin = runtime
            .timer_effects
            .registered_origin(first_wake.id)
            .expect("first auxiliary timer registration");
        assert!(matches!(
            first_origin,
            EffectOrigin::Auxiliary(actual) if actual.is_same_generation(&owner)
        ));

        let first_outcome = runtime.drain_runtime_messages();
        assert_eq!(first_outcome.messages_dispatched, 1);
        assert_eq!(runtime.bridge().dispatched, [1, 2]);

        let second_wake = runtime.bridge().scheduled[0];
        let second_origin = runtime
            .timer_effects
            .registered_origin(second_wake.id)
            .expect("chained auxiliary timer registration");
        assert!(matches!(
            second_origin,
            EffectOrigin::Auxiliary(actual) if actual.is_same_generation(&owner)
        ));

        let second_outcome = runtime.drain_runtime_messages();
        assert_eq!(second_outcome.messages_dispatched, 1);
        assert_eq!(runtime.bridge().dispatched, [1, 2, 3]);
        assert!(runtime.timer_effects.is_empty());
    }

    #[test]
    fn auxiliary_timer_retirement_drops_registration_and_fences_late_wake() {
        let mut runtime = SurfaceRuntime::new(
            TimerOriginBridge {
                scheduled: Vec::new(),
                dispatched: Vec::new(),
            },
            Vector2::new(80.0, 40.0),
        );
        let owner = runtime.acquire_auxiliary_effect_owner("settings");
        let _ = runtime.dispatch_message_from_auxiliary(1, owner.clone());
        let late_wake = runtime.bridge().scheduled[0];

        assert!(runtime.retire_auxiliary_effect_owner(&owner));
        assert!(runtime.timer_effects.is_empty());
        runtime.bridge_mut().scheduled.push(late_wake);

        let outcome = runtime.drain_runtime_messages();
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.bridge().dispatched, [1]);
        assert!(!runtime.retire_auxiliary_effect_owner(&owner));
    }

    #[test]
    fn stale_auxiliary_retirement_does_not_remove_new_same_key_timer() {
        let mut runtime = SurfaceRuntime::new(
            TimerOriginBridge {
                scheduled: Vec::new(),
                dispatched: Vec::new(),
            },
            Vector2::new(80.0, 40.0),
        );
        let old_owner = runtime.acquire_auxiliary_effect_owner("settings");
        let _ = runtime.dispatch_message_from_auxiliary(1, old_owner.clone());
        let old_wake = runtime.bridge().scheduled[0];
        assert!(runtime.retire_auxiliary_effect_owner(&old_owner));

        let new_owner = runtime.acquire_auxiliary_effect_owner("settings");
        let _ = runtime.dispatch_message_from_auxiliary(1, new_owner.clone());
        let new_wake = runtime.bridge().scheduled[1];
        assert!(!old_owner.is_same_generation(&new_owner));
        assert!(!runtime.retire_auxiliary_effect_owner(&old_owner));
        assert!(runtime.timer_effects.contains_registration(new_wake.id));

        let _ = runtime.drain_runtime_messages();
        assert_eq!(runtime.bridge().dispatched, [1, 1, 2]);
        let _ = runtime.drain_runtime_messages();
        assert_eq!(runtime.bridge().dispatched, [1, 1, 2, 3]);
        assert!(runtime.timer_effects.is_empty());
        assert_ne!(old_wake.id, new_wake.id);
    }

    #[test]
    fn auxiliary_timer_survives_native_recovery_without_retirement() {
        let mut runtime = SurfaceRuntime::new(
            TimerOriginBridge {
                scheduled: Vec::new(),
                dispatched: Vec::new(),
            },
            Vector2::new(80.0, 40.0),
        );
        let owner = runtime.acquire_auxiliary_effect_owner("settings");
        let _ = runtime.dispatch_message_from_auxiliary(1, owner.clone());
        let wake = runtime.bridge().scheduled[0];
        assert!(runtime.timer_effects.contains_registration(wake.id));

        assert!(runtime.begin_native_recovery());
        assert!(runtime.timer_effects.contains_registration(wake.id));
        assert!(runtime.finish_native_recovery());

        let _ = runtime.drain_runtime_messages();
        assert_eq!(runtime.bridge().dispatched, [1, 2]);
        assert!(runtime.auxiliary_effect_owner_is_active(&owner));
    }

    #[test]
    fn cancellation_and_panic_remove_mapper_without_invocation() {
        let mut effects = WorkerEffects::<usize>::default();
        let invoked = Arc::new(AtomicUsize::new(0));
        effects.registry.insert(
            EffectId(3),
            Registered {
                generation: EffectGeneration(1),
                registration_id: 0,
                epoch: effects.epoch,
                is_cancelled: None,
                lifecycle: LifecycleDescriptor::new(effects.owner.clone(), 3, None, 1, None),
                mapper: RegisteredMapper::Once(Box::new({
                    let invoked = Arc::clone(&invoked);
                    move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        Some(1)
                    }
                })),
                origin: EffectOrigin::Application,
            },
        );
        effects.pending += 1;
        assert!(effects.ingress.send(
            EffectId(3),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Cancelled,
        ));
        assert!(effects.drain().is_empty());
        assert_eq!(invoked.load(Ordering::Acquire), 0);
        assert!(!effects.registry.contains_key(&EffectId(3)));

        register(&mut effects, 4, 1);
        assert!(effects.ingress.send(
            EffectId(4),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Panicked(String::from("boom")),
        ));
        assert!(effects.drain().is_empty());
        assert_eq!(effects.pending, 0);
        assert!(!effects.registry.contains_key(&EffectId(4)));
    }

    #[test]
    fn cancellation_after_terminal_enqueue_skips_mapper() {
        let mut effects = WorkerEffects::<usize>::default();
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked = Arc::new(AtomicUsize::new(0));
        effects.registry.insert(
            EffectId(6),
            Registered {
                generation: EffectGeneration(1),
                registration_id: 0,
                epoch: effects.epoch,
                is_cancelled: {
                    let cancelled = Arc::clone(&cancelled);
                    Some(Arc::new(move || cancelled.load(Ordering::Acquire)))
                },
                lifecycle: LifecycleDescriptor::new(effects.owner.clone(), 6, None, 1, None),
                mapper: RegisteredMapper::Once(Box::new({
                    let invoked = Arc::clone(&invoked);
                    move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        Some(1)
                    }
                })),
                origin: EffectOrigin::Application,
            },
        );
        effects.pending += 1;
        assert!(effects.ingress.send(
            EffectId(6),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Completed(Box::new(13_usize)),
        ));
        cancelled.store(true, Ordering::Release);
        assert!(effects.drain().is_empty());
        assert_eq!(invoked.load(Ordering::Acquire), 0);
        assert_eq!(effects.pending, 0);
        assert!(effects.registry.is_empty());
    }

    #[test]
    fn completed_effect_invokes_and_drops_non_send_ui_mapper() {
        let mut effects = WorkerEffects::<usize>::default();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapper_state = Rc::clone(&mapped);
        let marker = Rc::new(());
        let mapper_marker = Rc::clone(&marker);
        effects.registry.insert(
            EffectId(8),
            Registered {
                generation: EffectGeneration(1),
                registration_id: 0,
                epoch: effects.epoch,
                is_cancelled: None,
                lifecycle: LifecycleDescriptor::new(effects.owner.clone(), 8, None, 1, None),
                mapper: RegisteredMapper::Once(Box::new(move |output| {
                    let _marker = &mapper_marker;
                    let output = *output.downcast::<usize>().expect("usize output");
                    mapper_state.borrow_mut().push(output);
                    Some(output + 1)
                })),
                origin: EffectOrigin::Application,
            },
        );
        effects.pending += 1;
        assert_eq!(Rc::strong_count(&marker), 2);
        assert!(effects.ingress.send(
            EffectId(8),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Completed(Box::new(7_usize)),
        ));

        assert_eq!(effects.drain(), vec![8]);
        assert_eq!(*mapped.borrow(), vec![7]);
        assert_eq!(Rc::strong_count(&marker), 1);
    }

    #[test]
    fn ordered_stream_maps_events_and_final_on_ui_in_fifo_order() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        runtime.execute_command(
            crate::runtime::Command::perform_worker_stream_with_priority(
                "ordered-stream",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest: false,
                },
                |sink| {
                    assert!(sink.emit(Box::new(1_u8)));
                    assert!(sink.emit(Box::new(2_u8)));
                    3_u8
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |event: u8| {
                        mapped.borrow_mut().push(event);
                        usize::from(event)
                    }
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |output: u8| {
                        mapped.borrow_mut().push(output);
                        usize::from(output)
                    }
                },
            ),
        );

        runtime.drain_runtime_messages();

        assert_eq!(*mapped.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn admission_receipt_resolves_after_inline_host_acceptance_before_output() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let receipt = crate::application::runtime::BusinessTaskAdmissionReceipt::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        runtime.execute_command(
            crate::runtime::Command::perform_worker_effect_with_priority_and_receipt(
                "receipt-inline",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                Some(crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard(receipt.weak())),
                || 7_u8,
                move |output| {
                    mapped_state.borrow_mut().push(output);
                    usize::from(output)
                },
            ),
        );
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert!(mapped.borrow().is_empty());
        runtime.drain_runtime_messages();
        assert_eq!(*mapped.borrow(), vec![7]);
    }

    #[test]
    fn admission_receipt_resolves_rejected_without_retrying() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut runtime = SurfaceRuntime::new(
            ToggleBridge {
                accepted: Arc::clone(&accepted),
            },
            Vector2::new(80.0, 40.0),
        );
        let receipt = crate::application::runtime::BusinessTaskAdmissionReceipt::new();
        runtime.execute_command(
            crate::runtime::Command::perform_worker_effect_with_priority_and_receipt(
                "receipt-rejected",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                Some(crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard(receipt.weak())),
                || 7_u8,
                |_| 7_usize,
            ),
        );
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert!(!runtime.drain_runtime_messages().runtime_work_remaining);
    }

    #[test]
    fn fenced_command_closes_pending_admission_receipt() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let receipt = crate::application::runtime::BusinessTaskAdmissionReceipt::new();
        let effect = crate::runtime::Command::perform_worker_effect_with_priority_and_receipt(
            "receipt-closed",
            crate::runtime::TaskPriority::Background,
            None,
            0,
            Some(crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard(receipt.weak())),
            || 7_u8,
            |_| 7_usize,
        );
        runtime.execute_command(crate::runtime::Command::batch(vec![
            crate::runtime::Command::exit(),
            effect,
        ]));
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Closed
        );
    }

    #[test]
    fn repeated_host_rejections_do_not_consume_admission_capacity() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut runtime = SurfaceRuntime::new(
            ToggleBridge {
                accepted: Arc::clone(&accepted),
            },
            Vector2::new(80.0, 40.0),
        );
        for _ in 0..(EFFECT_INGRESS_CAPACITY + 16) {
            let receipt = crate::application::runtime::BusinessTaskAdmissionReceipt::new();
            runtime.execute_command(
                crate::runtime::Command::perform_worker_effect_with_priority_and_receipt(
                    "receipt-rejected-burst",
                    crate::runtime::TaskPriority::Background,
                    None,
                    0,
                    Some(crate::application::runtime::update_context::business::admission::AdmissionReceiptGuard(receipt.weak())),
                    || 7_u8,
                    |_| 7_usize,
                ),
            );
            assert_eq!(
                receipt.poll(),
                crate::application::runtime::BusinessTaskAdmission::Rejected
            );
        }
        assert_eq!(runtime.worker_effects.pending, 0);
    }

    #[test]
    fn latest_stream_coalesces_events_and_keeps_final_after_latest_event() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        runtime.execute_command(
            crate::runtime::Command::perform_worker_stream_with_priority(
                "latest-stream",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest: true,
                },
                |sink| {
                    assert!(sink.emit_latest(Box::new(1_u8)));
                    assert!(sink.emit_latest(Box::new(2_u8)));
                    sink.close_latest();
                    assert!(!sink.emit_latest(Box::new(3_u8)));
                    4_u8
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |event: u8| {
                        mapped.borrow_mut().push(event);
                        usize::from(event)
                    }
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |output: u8| {
                        mapped.borrow_mut().push(output);
                        usize::from(output)
                    }
                },
            ),
        );

        runtime.drain_runtime_messages();

        assert_eq!(*mapped.borrow(), vec![2, 4]);
        let diagnostics = runtime.diagnostics.snapshot();
        assert_eq!(diagnostics.queue.stream_events_coalesced, 1);
        assert_eq!(diagnostics.queue.stream_events_stale, 1);
    }

    #[test]
    fn ordered_stream_pressure_drops_events_but_preserves_accepted_order_and_final() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        runtime.execute_command(
            crate::runtime::Command::perform_worker_stream_with_priority(
                "ordered-pressure",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest: false,
                },
                |sink| {
                    for event in 0..(EFFECT_INGRESS_CAPACITY + 8) {
                        let _ = sink.emit(Box::new(event as u8));
                    }
                    255_u8
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |event: u8| {
                        mapped.borrow_mut().push(event);
                        usize::from(event)
                    }
                },
                {
                    let mapped = Rc::clone(&mapped);
                    move |output: u8| {
                        mapped.borrow_mut().push(output);
                        usize::from(output)
                    }
                },
            ),
        );

        let first = runtime.drain_runtime_messages();
        assert!(first.runtime_work_remaining);
        let second = runtime.drain_runtime_messages();
        assert!(!second.runtime_work_remaining);

        let mapped = mapped.borrow();
        assert_eq!(mapped.len(), EFFECT_INGRESS_CAPACITY + 1);
        let expected = (0..EFFECT_INGRESS_CAPACITY as u8).collect::<Vec<_>>();
        assert_eq!(&mapped[..EFFECT_INGRESS_CAPACITY], expected.as_slice());
        assert_eq!(mapped.last(), Some(&255));
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_dropped,
            8
        );
        let diagnostics = runtime.diagnostics.snapshot();
        assert_eq!(diagnostics.queue.current_pending_controller_completions, 0);
        assert_eq!(diagnostics.queue.max_pending_controller_completions, 1);
        assert_eq!(diagnostics.queue.controller_completion_deferrals, 1);
    }

    #[test]
    fn controller_completion_budget_is_eight_during_active_drag() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        for id in 0..16_usize {
            let mapped = Rc::clone(&mapped);
            runtime.execute_command(
                crate::runtime::Command::perform_worker_effect_with_priority(
                    "active-drag-budget",
                    crate::runtime::TaskPriority::Background,
                    None,
                    id as u64,
                    move || id,
                    move |output| {
                        mapped.borrow_mut().push(output);
                        output
                    },
                ),
            );
        }
        runtime.execute_command(crate::runtime::Command::begin_drag(DragRequest::new(
            DragPreview::sized("dragging", Vector2::new(80.0, 20.0)),
            Point::new(0.0, 0.0),
        )));

        let first = runtime.drain_runtime_messages();
        assert_eq!(first.messages_dispatched, 8);
        assert!(first.runtime_work_remaining);
        assert_eq!(*mapped.borrow(), (0..8).collect::<Vec<_>>());
    }

    fn queue_external_completion(
        runtime: &mut SurfaceRuntime<ImmediateBridge, usize>,
        mapped: &Rc<RefCell<Vec<usize>>>,
    ) {
        let mapped = Rc::clone(mapped);
        runtime.execute_command(crate::runtime::Command::begin_external_drag(
            ExternalDragRequest::files([std::path::PathBuf::from("kick.wav")], "kick.wav"),
            move |_| {
                mapped.borrow_mut().push(100);
                100
            },
        ));
        let launch = runtime
            .take_external_drag_launch()
            .expect("external drag launch");
        runtime.dispatch_external_drag_launch_result(
            launch.identity,
            Ok(ExternalDragOutcome {
                effect: ExternalDragEffect::Copy,
            }),
        );
    }

    fn queue_platform_completions(
        runtime: &mut SurfaceRuntime<ImmediateBridge, usize>,
        mapped: &Rc<RefCell<Vec<usize>>>,
        count: usize,
    ) {
        for id in 0..count {
            let mapped = Rc::clone(mapped);
            let identity = runtime.platform_registry.register(
                Box::new(move |_| {
                    mapped.borrow_mut().push(id);
                    id
                }),
                &EffectOrigin::Application,
            );
            let reservation = crate::runtime::controller::platform::PlatformResultIngress::reserve(
                &runtime.platform_results,
            )
            .expect("platform completion reservation");
            assert!(reservation.commit(PlatformResultDelivery::Completed {
                identity,
                result: Err(String::from("test")),
            }));
        }
    }

    #[test]
    fn external_completion_shares_ordinary_budget_and_retains_the_remainder() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        queue_external_completion(&mut runtime, &mapped);
        queue_platform_completions(&mut runtime, &mapped, 64);

        let first = runtime.drain_runtime_messages();
        assert_eq!(first.messages_dispatched, 64);
        assert!(first.runtime_work_remaining);
        assert_eq!(mapped.borrow().first(), Some(&100));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(mapped.borrow().last(), Some(&63));
    }

    #[test]
    fn external_completion_shares_interactive_budget_and_retains_the_remainder() {
        let mut runtime = SurfaceRuntime::new(ImmediateBridge, Vector2::new(80.0, 40.0));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        queue_external_completion(&mut runtime, &mapped);
        queue_platform_completions(&mut runtime, &mapped, 8);
        runtime.execute_command(crate::runtime::Command::begin_drag(DragRequest::new(
            DragPreview::sized("dragging", Vector2::new(80.0, 20.0)),
            Point::new(0.0, 0.0),
        )));

        let first = runtime.drain_runtime_messages();
        assert_eq!(first.messages_dispatched, 8);
        assert!(first.runtime_work_remaining);
        assert_eq!(mapped.borrow().first(), Some(&100));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(mapped.borrow().last(), Some(&7));
    }

    #[test]
    fn shutdown_invalidates_epoch_and_clears_registry() {
        let mut effects = WorkerEffects::<usize>::default();
        register(&mut effects, 5, 1);
        let epoch = effects.epoch;
        effects.shutdown();
        assert!(effects.epoch > epoch);
        assert_eq!(effects.pending, 0);
        assert!(effects.registry.is_empty());
        assert!(effects.ingress.send(
            EffectId(5),
            EffectGeneration(1),
            epoch,
            EffectResult::Completed(Box::new(11_usize)),
        ));
        assert!(effects.drain().is_empty());
    }

    #[test]
    fn shutdown_disconnects_old_ingress_before_new_admission() {
        let mut effects = WorkerEffects::<usize>::default();
        let old_epoch = effects.epoch;
        let old_ingress = EffectIngress {
            owner: effects.owner.clone(),
            sender: effects.ingress.sender.clone(),
            sequence: Arc::clone(&effects.ingress.sequence),
            finals: Arc::clone(&effects.ingress.finals),
            stream_events_coalesced: Arc::clone(&effects.ingress.stream_events_coalesced),
            stream_events_dropped: Arc::clone(&effects.ingress.stream_events_dropped),
            stream_events_stale: Arc::clone(&effects.ingress.stream_events_stale),
        };
        effects.shutdown();

        register(&mut effects, 7, 1);
        assert!(!old_ingress.send(
            EffectId(70),
            EffectGeneration(1),
            old_epoch,
            EffectResult::Completed(Box::new(70_usize)),
        ));
        effects.deferred.push_back(EffectTerminal {
            sequence: 0,
            id: EffectId(70),
            generation: EffectGeneration(1),
            registration_id: 0,
            epoch: old_epoch,
            result: EffectResult::Completed(Box::new(70_usize)),
            owner: effects.owner.clone(),
            origin: EffectOrigin::Application,
        });
        assert!(effects.ingress.send(
            EffectId(7),
            EffectGeneration(1),
            effects.epoch,
            EffectResult::Completed(Box::new(7_usize)),
        ));
        assert_eq!(effects.drain(), vec![7]);
        assert_eq!(effects.pending, 0);
    }

    #[test]
    fn bounded_admission_refuses_more_than_ingress_capacity() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "capacity",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                move |_| id,
            );
            assert!(!runtime.execute_command(command).runtime_work_remaining);
        }
        assert_eq!(accepted.load(Ordering::Acquire), EFFECT_INGRESS_CAPACITY);
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(
            runtime.worker_effects.registry.len(),
            EFFECT_INGRESS_CAPACITY
        );
        let command = crate::runtime::Command::perform_worker_effect_with_priority(
            "capacity-overflow",
            crate::runtime::TaskPriority::Background,
            None,
            0,
            || 65,
            |_| 65,
        );
        let _ = runtime.execute_command(command);
        assert_eq!(accepted.load(Ordering::Acquire), EFFECT_INGRESS_CAPACITY);
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(
            runtime.worker_effects.registry.len(),
            EFFECT_INGRESS_CAPACITY
        );
    }

    struct DropProbe(Arc<AtomicUsize>);

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }

    #[test]
    fn latest_worker_capacity_rejection_rolls_back_idle_publication() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "capacity",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                move |_| id,
            );
            let _ = runtime.execute_command(command);
        }

        let mut latest = crate::application::LatestTask::new();
        let transaction = latest.begin_replacement();
        let probe = Arc::new(AtomicUsize::new(0));
        let probe_guard = DropProbe(Arc::clone(&probe));
        let command = crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
            EffectId(900),
            "latest-capacity",
            crate::runtime::TaskPriority::Background,
            None,
            transaction.generation(),
            Some(transaction),
            || 1_u8,
            move |_| {
                let _probe = probe_guard;
                1_usize
            },
        );
        let _ = runtime.execute_command(command);
        assert_eq!(latest.active(), None);
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(probe.load(Ordering::Acquire), 1);
    }

    #[test]
    fn latest_worker_host_rejection_restores_predecessor_mapper_and_ticket() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let bridge = ToggleBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = crate::application::LatestTask::new();

        let first = latest.begin_replacement();
        let first_ticket = first.replacement();
        let first_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(901),
                "latest-first",
                crate::runtime::TaskPriority::Background,
                None,
                first.generation(),
                Some(first),
                || 1_u8,
                |_| 11_usize,
            );
        let _ = runtime.execute_command(first_command);

        let second = latest.begin_replacement();
        let second_ticket = second.replacement();
        let probe = Arc::new(AtomicUsize::new(0));
        let probe_guard = DropProbe(Arc::clone(&probe));
        let second_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(901),
                "latest-second",
                crate::runtime::TaskPriority::Background,
                None,
                second.generation(),
                Some(second),
                || 2_u8,
                move |_| {
                    let _probe = probe_guard;
                    22_usize
                },
            );
        accepted.store(false, Ordering::Release);
        let _ = runtime.execute_command(second_command);

        assert_eq!(latest.active(), Some(first_ticket));
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(probe.load(Ordering::Acquire), 1);
        assert_eq!(runtime.worker_effects.pending, 1);
        assert!(runtime.worker_effects.registry.contains_key(&EffectId(901)));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(901),
            EffectGeneration(first_ticket.id()),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(3_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![11]);
    }

    #[test]
    fn discarded_latest_worker_command_rolls_back_and_releases_mapper_capture() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let bridge = ToggleBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = crate::application::LatestTask::new();

        let first = latest.begin_replacement();
        let first_ticket = first.replacement();
        let first_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(903),
                "latest-first",
                crate::runtime::TaskPriority::Background,
                None,
                first.generation(),
                Some(first),
                || 1_u8,
                |_| 11_usize,
            );
        let _ = runtime.execute_command(first_command);

        let second = latest.begin_replacement();
        let probe = Arc::new(AtomicUsize::new(0));
        let probe_guard = DropProbe(Arc::clone(&probe));
        let second_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(903),
                "latest-discarded",
                crate::runtime::TaskPriority::Background,
                None,
                second.generation(),
                Some(second),
                || 2_u8,
                move |_| {
                    let _probe = probe_guard;
                    22_usize
                },
            );
        drop(second_command);

        assert_eq!(latest.active(), Some(first_ticket));
        assert_eq!(probe.load(Ordering::Acquire), 1);
        assert_eq!(runtime.worker_effects.pending, 1);
        assert!(runtime.worker_effects.ingress.send(
            EffectId(903),
            EffectGeneration(first_ticket.id()),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(3_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![11]);
    }

    #[test]
    fn latest_worker_acceptance_fences_predecessor_terminal_mapper() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = crate::application::LatestTask::new();
        let first = latest.begin_replacement();
        let first_generation = first.generation();
        let first_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(902),
                "latest-first",
                crate::runtime::TaskPriority::Background,
                None,
                first_generation,
                Some(first),
                || 1_u8,
                |_| 1_usize,
            );
        let _ = runtime.execute_command(first_command);
        let second = latest.begin_replacement();
        let second_generation = second.generation();
        let second_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(902),
                "latest-second",
                crate::runtime::TaskPriority::Background,
                None,
                second_generation,
                Some(second),
                || 2_u8,
                |_| 2_usize,
            );
        let _ = runtime.execute_command(second_command);
        assert!(runtime.worker_effects.ingress.send(
            EffectId(902),
            EffectGeneration(first_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(1_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(902),
            EffectGeneration(second_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(2_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![2]);
    }

    #[test]
    fn keyed_latest_worker_acceptance_fences_stale_event_and_final() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = crate::application::KeyedLatestTasks::new();
        let (first_ticket, first, effect_id) = keyed.begin_replacement(7_u32);
        let first_generation = first.generation();
        let first_command =
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                EffectId(effect_id),
                "keyed-first",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: first_generation,
                    latest: false,
                },
                Some(first),
                |_sink| 1_u8,
                |_: u8| 10_usize,
                |_: u8| 11_usize,
            );
        let _ = runtime.execute_command(first_command);

        let (second_ticket, second, second_effect_id) = keyed.begin_replacement(7_u32);
        assert_eq!(effect_id, second_effect_id);
        assert_ne!(first_ticket, second_ticket);
        let second_generation = second.generation();
        let second_command =
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                EffectId(second_effect_id),
                "keyed-second",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: second_generation,
                    latest: false,
                },
                Some(second),
                |_sink| 2_u8,
                |_: u8| 20_usize,
                |_: u8| 21_usize,
            );
        let _ = runtime.execute_command(second_command);

        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(first_generation),
            runtime.worker_effects.epoch,
            EffectResult::Event(Box::new(1_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(first_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(1_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(second_generation),
            runtime.worker_effects.epoch,
            EffectResult::Event(Box::new(2_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(second_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(2_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![20, 21]);
        assert_eq!(keyed.active(&7_u32), Some(second_ticket));
    }

    #[test]
    fn keyed_latest_worker_host_rejection_restores_only_that_key() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let bridge = ToggleBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = crate::application::KeyedLatestTasks::new();
        let (first_ticket, first, effect_id) = keyed.begin_replacement(1_u8);
        let first_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(effect_id),
                "keyed-first",
                crate::runtime::TaskPriority::Background,
                None,
                first.generation(),
                Some(first),
                || 1_u8,
                |_| 11_usize,
            );
        let _ = runtime.execute_command(first_command);
        let (_second_ticket, second, second_effect_id) = keyed.begin_replacement(1_u8);
        let (other_ticket, other, other_effect_id) = keyed.begin_replacement(2_u8);
        assert_eq!(effect_id, second_effect_id);
        assert_ne!(effect_id, other_effect_id);
        let probe = Arc::new(AtomicUsize::new(0));
        let probe_guard = DropProbe(Arc::clone(&probe));
        let second_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(second_effect_id),
                "keyed-rejected",
                crate::runtime::TaskPriority::Background,
                None,
                second.generation(),
                Some(second),
                || 2_u8,
                move |_| {
                    let _probe = probe_guard;
                    22_usize
                },
            );
        let other_command =
            crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
                EffectId(other_effect_id),
                "keyed-other",
                crate::runtime::TaskPriority::Background,
                None,
                other.generation(),
                Some(other),
                || 3_u8,
                |_| 33_usize,
            );
        accepted.store(false, Ordering::Release);
        let _ = runtime.execute_command(second_command);
        accepted.store(true, Ordering::Release);
        let _ = runtime.execute_command(other_command);
        assert_eq!(keyed.active(&1_u8), Some(first_ticket));
        assert_eq!(keyed.active(&2_u8), Some(other_ticket));
        assert_eq!(probe.load(Ordering::Acquire), 1);
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(first_ticket.id()),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(1_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![11]);
    }

    #[test]
    fn keyed_latest_worker_capacity_rejection_rolls_back_its_key() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "keyed-capacity",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                move |_| id,
            );
            let _ = runtime.execute_command(command);
        }
        let mut keyed = crate::application::KeyedLatestTasks::new();
        let (_ticket, transaction, effect_id) = keyed.begin_replacement(9_u8);
        let command = crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
            EffectId(effect_id),
            "keyed-capacity-overflow",
            crate::runtime::TaskPriority::Background,
            None,
            transaction.generation(),
            Some(transaction),
            || 1_u8,
            |_| 1_usize,
        );
        let _ = runtime.execute_command(command);
        assert_eq!(keyed.active(&9_u8), None);
    }

    #[test]
    fn resource_exclusive_worker_host_rejection_releases_only_that_reservation() {
        let accepted = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let bridge = ToggleBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let key = crate::runtime::ResourceKey::scoped("sample", "C:/rejected-exclusive.wav");
        let mut resources = crate::application::ResourceTasks::new();
        let (ticket, transaction, effect_id) = resources
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive admission");
        let generation = transaction.generation();
        let mapper_dropped = Arc::new(AtomicUsize::new(0));
        let mapper_guard = DropProbe(Arc::clone(&mapper_dropped));
        let command = crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
            EffectId(effect_id),
            "resource-exclusive-rejected",
            crate::runtime::TaskPriority::Background,
            None,
            generation,
            Some(transaction),
            || 1_u8,
            move |_| {
                let _mapper_guard = mapper_guard;
                1_usize
            },
        );

        accepted.store(false, Ordering::Release);
        let _ = runtime.execute_command(command);

        assert_eq!(resources.active(&key), None);
        assert!(!resources.is_active_key(&key, ticket.ticket()));
        assert_eq!(mapper_dropped.load(Ordering::Acquire), 1);
        let (_replacement, replacement_transaction, _) = resources
            .begin_exclusive_transaction(key)
            .expect("host rejection should release this key");
        replacement_transaction.reject();
    }

    #[test]
    fn resource_exclusive_worker_capacity_rejection_releases_only_that_reservation() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "resource-exclusive-capacity",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                move |_| id,
            );
            let _ = runtime.execute_command(command);
        }

        let key = crate::runtime::ResourceKey::scoped("sample", "C:/capacity-exclusive.wav");
        let mut resources = crate::application::ResourceTasks::new();
        let (ticket, transaction, effect_id) = resources
            .begin_exclusive_transaction(key.clone())
            .expect("exclusive admission");
        let command = crate::runtime::Command::perform_worker_effect_with_identity_and_transaction(
            EffectId(effect_id),
            "resource-exclusive-capacity-overflow",
            crate::runtime::TaskPriority::Background,
            None,
            transaction.generation(),
            Some(transaction),
            || 1_u8,
            |_| 1_usize,
        );

        let _ = runtime.execute_command(command);

        assert_eq!(resources.active(&key), None);
        assert!(!resources.is_active_key(&key, ticket.ticket()));
        let (_replacement, replacement_transaction, _) = resources
            .begin_exclusive_transaction(key)
            .expect("capacity rejection should release this key");
        replacement_transaction.reject();
    }

    #[test]
    fn resource_latest_worker_acceptance_fences_stale_event_and_final() {
        let accepted = Arc::new(AtomicUsize::new(0));
        let bridge = AdmissionBridge {
            accepted: Arc::clone(&accepted),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let key = crate::runtime::ResourceKey::scoped("sample", "C:/resource.wav");
        let mut resources = crate::application::ResourceTasks::new();
        let (first_ticket, first, effect_id) = resources.begin_latest_transaction(key.clone());
        let first_generation = first.generation();
        let first_command =
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                EffectId(effect_id),
                "resource-first",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: first_generation,
                    latest: false,
                },
                Some(first),
                |_sink| 1_u8,
                |_: u8| 10_usize,
                |_: u8| 11_usize,
            );
        let _ = runtime.execute_command(first_command);
        let (second_ticket, second, second_effect_id) =
            resources.begin_latest_transaction(key.clone());
        assert_eq!(effect_id, second_effect_id);
        let second_generation = second.generation();
        let second_command =
            crate::runtime::Command::perform_worker_stream_with_identity_and_transaction(
                EffectId(second_effect_id),
                "resource-second",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: second_generation,
                    latest: false,
                },
                Some(second),
                |_sink| 2_u8,
                |_: u8| 20_usize,
                |_: u8| 21_usize,
            );
        let _ = runtime.execute_command(second_command);
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(first_generation),
            runtime.worker_effects.epoch,
            EffectResult::Event(Box::new(1_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(first_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(1_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(second_generation),
            runtime.worker_effects.epoch,
            EffectResult::Event(Box::new(2_u8)),
        ));
        assert!(runtime.worker_effects.ingress.send(
            EffectId(effect_id),
            EffectGeneration(second_generation),
            runtime.worker_effects.epoch,
            EffectResult::Completed(Box::new(2_u8)),
        ));
        assert_eq!(runtime.worker_effects.drain(), vec![20, 21]);
        assert_eq!(resources.active(&key), Some(second_ticket.ticket()));
        assert_ne!(first_ticket.ticket(), second_ticket.ticket());
    }

    struct OriginBridge {
        show_owner: bool,
    }

    impl crate::runtime::RuntimeBridge<usize> for OriginBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            if self.show_owner {
                crate::runtime::test_arc_surface(
                    column([text::<usize>("keyed").key("keyed")]).into_surface(),
                )
            } else {
                crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    Vec::new(),
                )))
            }
        }

        fn update(&mut self, message: usize) -> crate::runtime::Command<usize> {
            if message > 2 {
                return crate::runtime::Command::none();
            }
            crate::runtime::Command::perform_worker_effect_with_identity(
                EffectId(message as u64),
                "auxiliary-origin-chain",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || message,
                |output| output + 1,
            )
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for OriginBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            _work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            true
        }
    }

    struct TimerOriginBridge {
        scheduled: Vec<RuntimeTimerWake>,
        dispatched: Vec<usize>,
    }

    impl crate::runtime::RuntimeBridge<usize> for TimerOriginBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn reduce_message(&mut self, message: usize) {
            self.dispatched.push(message);
        }

        fn update(&mut self, message: usize) -> crate::runtime::Command<usize> {
            self.dispatched.push(message);
            match message {
                1 | 2 => crate::runtime::Command::after(Duration::ZERO, message + 1),
                _ => crate::runtime::Command::none(),
            }
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks().with_queues()
        }
    }

    impl RuntimeTaskHost<usize> for TimerOriginBridge {
        fn schedule_timer(&mut self, _delay: Duration, wake: RuntimeTimerWake) -> bool {
            self.scheduled.push(wake);
            true
        }
    }

    impl RuntimeQueueHost<usize> for TimerOriginBridge {
        fn take_runtime_timer_wakes(&mut self) -> Vec<RuntimeTimerWake> {
            std::mem::take(&mut self.scheduled)
        }
    }

    struct AdmissionBridge {
        accepted: Arc<AtomicUsize>,
    }

    struct ToggleBridge {
        accepted: Arc<std::sync::atomic::AtomicBool>,
    }

    #[derive(Default)]
    struct ImmediateBridge;

    impl crate::runtime::RuntimeBridge<usize> for ImmediateBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for ImmediateBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            work();
            true
        }
    }

    impl crate::runtime::RuntimeBridge<usize> for AdmissionBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for AdmissionBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            _work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            self.accepted.fetch_add(1, Ordering::AcqRel);
            true
        }
    }

    impl crate::runtime::RuntimeBridge<usize> for ToggleBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for ToggleBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            _work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            self.accepted.load(Ordering::Acquire)
        }
    }
}
