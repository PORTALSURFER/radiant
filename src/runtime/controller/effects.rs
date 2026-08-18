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
    rc::Rc,
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
        if !origin.is_live() {
            self.record_stale();
            return false;
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkerEffectMappingMode {
    Eager,
    DeferredOwnerLatestStream,
}

enum RegisteredMapper<Message> {
    Once(Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>),
    Stream {
        mapping_mode: WorkerEffectMappingMode,
        latest: bool,
        latest_state: Option<Arc<LatestStreamState>>,
        map_event: Rc<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
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
        mapping_mode: WorkerEffectMappingMode,
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
                    mapping_mode,
                    latest,
                    latest_state: None,
                    map_event: Rc::from(map_event),
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
                mapping_mode,
                latest,
                latest_state: _,
                map_event,
                map_final,
            } => RegisteredMapper::Stream {
                mapping_mode,
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
                        work(sink, is_cancelled.clone())
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
            .filter_map(|mapped| mapped.resolve(true).map(|(message, _origin)| message))
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
                if let RegisteredMapper::Stream {
                    mapping_mode,
                    map_event,
                    ..
                } = &entry.mapper
                {
                    let origin = entry.origin.clone();
                    match mapping_mode {
                        WorkerEffectMappingMode::Eager => {
                            if let Some(message) = map_event(output) {
                                messages.push(MappedEffectMessage::ready(message, origin));
                            }
                        }
                        WorkerEffectMappingMode::DeferredOwnerLatestStream => {
                            messages.push(MappedEffectMessage::deferred_event(
                                map_event.clone(),
                                output,
                                origin,
                                entry.is_cancelled.clone(),
                            ));
                        }
                    }
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
                    mapping_mode,
                    latest_state: Some(state),
                    map_event,
                    ..
                } = &entry.mapper
                {
                    let Some(output) = state.take_latest() else {
                        return;
                    };
                    let origin = entry.origin.clone();
                    match mapping_mode {
                        WorkerEffectMappingMode::Eager => {
                            if let Some(message) = map_event(output) {
                                messages.push(MappedEffectMessage::ready(message, origin));
                            }
                        }
                        WorkerEffectMappingMode::DeferredOwnerLatestStream => {
                            messages.push(MappedEffectMessage::deferred_event(
                                map_event.clone(),
                                output,
                                origin,
                                entry.is_cancelled.clone(),
                            ));
                        }
                    }
                }
            }
            EffectResult::Completed(output) => {
                let Some(entry) = self.registry.remove(&terminal.id) else {
                    return;
                };
                let cancellation_probe = entry.is_cancelled.clone();
                if !entry.origin.is_live()
                    || cancellation_probe.as_ref().is_some_and(|probe| probe())
                {
                    return;
                }
                let origin = entry.origin.clone();
                match entry.mapper {
                    RegisteredMapper::Once(map) => {
                        if let Some(message) = map(output) {
                            messages.push(MappedEffectMessage::ready(message, origin));
                        }
                    }
                    RegisteredMapper::Stream {
                        mapping_mode,
                        map_final,
                        ..
                    } => match mapping_mode {
                        WorkerEffectMappingMode::Eager => {
                            if let Some(message) = map_final(output) {
                                messages.push(MappedEffectMessage::ready(message, origin));
                            }
                        }
                        WorkerEffectMappingMode::DeferredOwnerLatestStream => {
                            messages.push(MappedEffectMessage::deferred(
                                map_final,
                                output,
                                origin,
                                cancellation_probe,
                            ));
                        }
                    },
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
    mapping: MappedEffect<Message>,
    origin: EffectOrigin,
}

enum MappedEffect<Message> {
    Ready(Message),
    Deferred {
        output: Box<dyn Any + Send>,
        mapper: Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        cancellation_probe: Option<CancellationProbe>,
    },
    DeferredEvent {
        output: Box<dyn Any + Send>,
        mapper: Rc<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        cancellation_probe: Option<CancellationProbe>,
    },
}

impl<Message> MappedEffectMessage<Message> {
    fn ready(message: Message, origin: EffectOrigin) -> Self {
        Self {
            mapping: MappedEffect::Ready(message),
            origin,
        }
    }

    fn deferred(
        mapper: Box<dyn FnOnce(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        output: Box<dyn Any + Send>,
        origin: EffectOrigin,
        cancellation_probe: Option<CancellationProbe>,
    ) -> Self {
        Self {
            mapping: MappedEffect::Deferred {
                output,
                mapper,
                cancellation_probe,
            },
            origin,
        }
    }

    fn deferred_event(
        mapper: Rc<dyn Fn(Box<dyn Any + Send>) -> Option<Message> + 'static>,
        output: Box<dyn Any + Send>,
        origin: EffectOrigin,
        cancellation_probe: Option<CancellationProbe>,
    ) -> Self {
        Self {
            mapping: MappedEffect::DeferredEvent {
                output,
                mapper,
                cancellation_probe,
            },
            origin,
        }
    }

    pub(in crate::runtime::controller) fn origin(&self) -> &EffectOrigin {
        &self.origin
    }

    pub(in crate::runtime::controller) fn resolve(
        self,
        allow_deferred: bool,
    ) -> Option<(Message, EffectOrigin)> {
        let Self { mapping, origin } = self;
        let message = match mapping {
            MappedEffect::Ready(message) => Some(message),
            MappedEffect::Deferred {
                output,
                mapper,
                cancellation_probe,
            } if allow_deferred => {
                if cancellation_probe.as_ref().is_some_and(|probe| probe()) {
                    None
                } else {
                    mapper(output)
                }
            }
            MappedEffect::DeferredEvent {
                output,
                mapper,
                cancellation_probe,
            } if allow_deferred => {
                if cancellation_probe.as_ref().is_some_and(|probe| probe()) {
                    None
                } else {
                    mapper(output)
                }
            }
            MappedEffect::Deferred { .. } => None,
            MappedEffect::DeferredEvent { .. } => None,
        }?;
        Some((message, origin))
    }
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
        mapping_mode: WorkerEffectMappingMode,
    ) -> bool {
        let mut effects = std::mem::take(&mut self.worker_effects);
        let accepted = effects.submit(self, effect, origin, mapping_mode);
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
        application::{
            DeclarativeEffectOwner, IntoView, KeyedLatestTasks, KeyedTaskCompletion, LatestTask,
            TaskCompletion, column, text,
        },
        gui::types::{Point, Vector2},
        runtime::SurfaceRuntime,
    };
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
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
                    mapping_mode: WorkerEffectMappingMode::Eager,
                    latest: true,
                    latest_state: Some(Arc::clone(&state)),
                    map_event: Rc::new(|output| {
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

    fn owner_surface(owner: DeclarativeEffectOwner) -> UiSurface<usize> {
        column([text::<usize>("owned").key("owned").effect_owner(owner)]).into_surface()
    }

    #[derive(Clone, Copy)]
    enum OwnerSurfaceMode {
        Single,
        Reordered,
        Ambiguous,
        Unkeyed,
        Incompatible,
    }

    struct OwnerWorkerBridge {
        owner: DeclarativeEffectOwner,
        show_owner: bool,
        accept_worker: bool,
        surface_mode: OwnerSurfaceMode,
        spawned: Arc<AtomicUsize>,
        retire_on_event: bool,
        final_reducer_hits: Arc<AtomicUsize>,
        reduced_messages: Arc<Mutex<Vec<usize>>>,
        close_on_event: bool,
        keyed_supersession: Option<KeyedLatestTasks<u8>>,
        defer_next_worker: bool,
        deferred_worker: Option<Box<dyn FnOnce() + Send + 'static>>,
        trace: Option<Arc<Mutex<Vec<&'static str>>>>,
    }

    impl OwnerWorkerBridge {
        fn new(owner: DeclarativeEffectOwner, accept_worker: bool) -> Self {
            Self {
                owner,
                show_owner: true,
                accept_worker,
                surface_mode: OwnerSurfaceMode::Single,
                spawned: Arc::new(AtomicUsize::new(0)),
                retire_on_event: false,
                final_reducer_hits: Arc::new(AtomicUsize::new(0)),
                reduced_messages: Arc::new(Mutex::new(Vec::new())),
                close_on_event: false,
                keyed_supersession: None,
                defer_next_worker: false,
                deferred_worker: None,
                trace: None,
            }
        }

        fn run_deferred_worker(&mut self) {
            if let Some(work) = self.deferred_worker.take() {
                work();
            }
        }
    }

    impl crate::runtime::RuntimeBridge<usize> for OwnerWorkerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            if self.show_owner {
                let surface = match self.surface_mode {
                    OwnerSurfaceMode::Single => owner_surface(self.owner),
                    OwnerSurfaceMode::Reordered => column([
                        text::<usize>("sibling").key("sibling"),
                        text::<usize>("owned").key("owned").effect_owner(self.owner),
                    ])
                    .into_surface(),
                    OwnerSurfaceMode::Ambiguous => column([
                        text::<usize>("first").key("first").effect_owner(self.owner),
                        text::<usize>("second")
                            .key("second")
                            .effect_owner(self.owner),
                    ])
                    .into_surface(),
                    OwnerSurfaceMode::Unkeyed => text::<usize>("unkeyed")
                        .effect_owner(self.owner)
                        .into_surface(),
                    OwnerSurfaceMode::Incompatible => column([text::<usize>("replacement")])
                        .key("owned")
                        .effect_owner(DeclarativeEffectOwner::new())
                        .into_surface(),
                };
                crate::runtime::test_arc_surface(surface)
            } else {
                crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    Vec::new(),
                )))
            }
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }

        fn update(&mut self, message: usize) -> crate::runtime::Command<usize> {
            self.reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(message);
            if let Some(trace) = self.trace.as_ref() {
                trace
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push("reduce");
            }
            if message == 1
                && let Some(keyed) = self.keyed_supersession.as_mut()
            {
                self.defer_next_worker = true;
                let mut context = crate::application::runtime::update_context::UiUpdateContext::<
                    usize,
                >::default();
                let _receipt = context
                    .business()
                    .background("owner-keyed-latest-coalesced-stream-superseding")
                    .latest_for(keyed, 7_u8)
                    .stream_latest_for_owner_with_receipt(
                        self.owner,
                        |_, events| {
                            assert!(events.emit(9_u8));
                            10_u8
                        },
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                    );
                return context.into_command();
            }
            if self.retire_on_event && message == 1 {
                self.show_owner = false;
            }
            if self.close_on_event && message == 1 {
                return crate::runtime::Command::exit();
            }
            if message == 2 {
                self.final_reducer_hits.fetch_add(1, Ordering::AcqRel);
            }
            crate::runtime::Command::none()
        }
    }

    impl RuntimeTaskHost<usize> for OwnerWorkerBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            if !self.accept_worker {
                return false;
            }
            self.spawned.fetch_add(1, Ordering::AcqRel);
            if self.defer_next_worker {
                self.defer_next_worker = false;
                self.deferred_worker = Some(work);
                return true;
            }
            work();
            true
        }
    }

    type PendingWork = Arc<Mutex<Option<Box<dyn FnOnce() + Send + 'static>>>>;

    struct DeferredOwnerBridge {
        owner: DeclarativeEffectOwner,
        show_owner: bool,
        pending_work: PendingWork,
    }

    impl crate::runtime::RuntimeBridge<usize> for DeferredOwnerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            if self.show_owner {
                crate::runtime::test_arc_surface(owner_surface(self.owner))
            } else {
                crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    Vec::new(),
                )))
            }
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for DeferredOwnerBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            *self
                .pending_work
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(work);
            true
        }
    }

    struct SameUpdateRemovedOwnerBridge {
        owner: DeclarativeEffectOwner,
        removed: bool,
        receipt: Option<crate::application::runtime::BusinessTaskAdmissionReceipt>,
        spawned: Arc<AtomicUsize>,
        latest: Option<LatestTask>,
        keyed_coalesced_stream: Option<KeyedLatestTasks<u8>>,
        keyed_stream: Option<KeyedLatestTasks<u8>>,
        keyed: Option<KeyedLatestTasks<u8>>,
    }

    impl crate::runtime::RuntimeBridge<usize> for SameUpdateRemovedOwnerBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            if self.removed {
                crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    Vec::new(),
                )))
            } else {
                crate::runtime::test_arc_surface(owner_surface(self.owner))
            }
        }

        fn update(&mut self, _message: usize) -> crate::runtime::Command<usize> {
            self.removed = true;
            let mut context =
                crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
            let receipt = if let Some(keyed_coalesced_stream) = self.keyed_coalesced_stream.as_mut()
            {
                context
                    .business()
                    .background("owner-keyed-latest-coalesced-stream-same-update-removed")
                    .latest_for(keyed_coalesced_stream, 7_u8)
                    .stream_latest_for_owner_with_receipt(
                        self.owner,
                        |_, events| {
                            assert!(events.emit(1_u8));
                            2_u8
                        },
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                    )
            } else if let Some(keyed_stream) = self.keyed_stream.as_mut() {
                context
                    .business()
                    .background("owner-keyed-latest-stream-same-update-removed")
                    .latest_for(keyed_stream, 7_u8)
                    .stream_for_owner_with_receipt(
                        self.owner,
                        |_, events| {
                            assert!(events.emit(1_u8));
                            2_u8
                        },
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                    )
            } else if let Some(keyed) = self.keyed.as_mut() {
                context
                    .business()
                    .background("owner-keyed-latest-same-update-removed")
                    .latest_for(keyed, 7_u8)
                    .run_for_owner_with_receipt(
                        self.owner,
                        |_| 2_u8,
                        |completion: KeyedTaskCompletion<u8, u8>| completion.output as usize,
                    )
            } else if let Some(latest) = self.latest.as_mut() {
                context
                    .business()
                    .background("owner-latest-coalesced-stream-same-update-removed")
                    .latest(latest)
                    .stream_latest_for_owner_with_receipt(
                        self.owner,
                        |_, events| {
                            assert!(events.emit(1_u8));
                            2_u8
                        },
                        |completion: TaskCompletion<u8>| completion.output as usize,
                        |completion: TaskCompletion<u8>| completion.output as usize,
                    )
            } else {
                context
                    .business()
                    .background("owner-stream-latest-same-update-removed")
                    .stream_latest_for_owner_with_receipt(
                        self.owner,
                        |_, events| {
                            assert!(events.emit(1_u8));
                            2_u8
                        },
                        |event: u8| event as usize,
                        |output: u8| output as usize,
                    )
            };
            self.receipt = Some(receipt);
            context.into_command()
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for SameUpdateRemovedOwnerBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: crate::runtime::TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            self.spawned.fetch_add(1, Ordering::AcqRel);
            work();
            true
        }
    }

    fn owner_latest_command(
        latest: &mut LatestTask,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        map: impl FnOnce(TaskCompletion<usize>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest(latest);
        let ticket = request.ticket();
        let receipt = request.run_for_owner_with_receipt(owner, |_| 7_usize, map);
        (context.into_command(), receipt, ticket)
    }

    fn owner_keyed_latest_command(
        latest: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        map: impl FnOnce(KeyedTaskCompletion<u8, usize>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
        u8,
    ) {
        owner_keyed_latest_command_with_work(latest, key, owner, name, |_| 7_usize, map)
    }

    fn owner_keyed_latest_command_with_work(
        latest: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(crate::application::runtime::BusinessWorkContext) -> usize + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<u8, usize>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
        u8,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest_for(latest, key);
        let ticket = request.ticket();
        let request_key = *request.key();
        let receipt = request.run_for_owner_with_receipt(owner, work, map);
        (context.into_command(), receipt, ticket, request_key)
    }

    fn owner_keyed_ordered_stream_command(
        latest: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(KeyedTaskCompletion<u8, u8>) -> usize + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<u8, u8>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
        u8,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest_for(latest, key);
        let ticket = request.ticket();
        let request_key = *request.key();
        let receipt = request.stream_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt, ticket, request_key)
    }

    fn owner_keyed_coalesced_stream_command(
        latest: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(KeyedTaskCompletion<u8, u8>) -> usize + 'static,
        map_final: impl FnOnce(KeyedTaskCompletion<u8, u8>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
        u8,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest_for(latest, key);
        let ticket = request.ticket();
        let request_key = *request.key();
        let receipt =
            request.stream_latest_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt, ticket, request_key)
    }

    fn owner_latest_ordered_stream_command(
        latest: &mut LatestTask,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(TaskCompletion<u8>) -> usize + 'static,
        map_final: impl FnOnce(TaskCompletion<u8>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest(latest);
        let ticket = request.ticket();
        let receipt = request.stream_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt, ticket)
    }

    fn owner_latest_coalesced_stream_command(
        latest: &mut LatestTask,
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(TaskCompletion<u8>) -> usize + 'static,
        map_final: impl FnOnce(TaskCompletion<u8>) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
        crate::application::TaskTicket,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let request = context.business().background(name).latest(latest);
        let ticket = request.ticket();
        let receipt =
            request.stream_latest_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt, ticket)
    }

    fn owner_ordered_stream_command(
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(u8) -> usize + 'static,
        map_final: impl FnOnce(u8) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let receipt = context
            .business()
            .background(name)
            .stream_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt)
    }

    fn owner_coalesced_stream_command(
        owner: DeclarativeEffectOwner,
        name: &'static str,
        work: impl FnOnce(
            crate::application::runtime::BusinessWorkContext,
            crate::application::runtime::BusinessEventSink<u8>,
        ) -> u8
        + Send
        + 'static,
        map_event: impl Fn(u8) -> usize + 'static,
        map_final: impl FnOnce(u8) -> usize + 'static,
    ) -> (
        crate::runtime::Command<usize>,
        crate::application::runtime::BusinessTaskAdmissionReceipt,
    ) {
        let mut context =
            crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
        let receipt = context
            .business()
            .background(name)
            .stream_latest_for_owner_with_receipt(owner, work, map_event, map_final);
        (context.into_command(), receipt)
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
    fn owner_latest_admission_accepts_and_maps_once() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, ticket) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-valid",
            move |completion| {
                mapped_state.borrow_mut().push(completion.ticket.id());
                completion.output
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![ticket.id()]);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![ticket.id()]);
    }

    #[test]
    fn owner_latest_invalid_owner_rolls_back_without_spawn_or_mapping() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement) = owner_latest_command(
            &mut latest,
            DeclarativeEffectOwner::new(),
            "owner-latest-invalid",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_admission_accepts_and_maps_one_exact_completion() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, usize)>::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-valid",
            move |completion| {
                mapped_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                completion.output
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(key, ticket.id(), 7)]);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![7]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    fn assert_owner_keyed_latest_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        keyed: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let predecessor = keyed.begin(key);
        let sibling_key = key.wrapping_add(1);
        let sibling = keyed.begin(sibling_key);
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt, replacement, request_key) =
            owner_keyed_latest_command(keyed, key, owner, name, move |_| {
                mapped_state.borrow_mut().push(1);
                1
            });

        let outcome = runtime.execute_command(command);
        assert_eq!(request_key, key);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&sibling_key), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert!(runtime.worker_effects.registry.is_empty());
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_rejects_invalid_removed_ambiguous_unkeyed_and_incompatible_handles() {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut invalid_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut invalid_runtime,
            &mut invalid_keyed,
            7,
            DeclarativeEffectOwner::new(),
            "owner-keyed-latest-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        let mut removed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut removed_runtime,
            &mut removed_keyed,
            7,
            owner,
            "owner-keyed-latest-removed",
        );

        let mut ambiguous_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        ambiguous_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Ambiguous;
        let mut ambiguous_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut ambiguous_runtime,
            &mut ambiguous_keyed,
            7,
            owner,
            "owner-keyed-latest-ambiguous",
        );

        let mut unkeyed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        unkeyed_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Unkeyed;
        let mut unkeyed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut unkeyed_runtime,
            &mut unkeyed_keyed,
            7,
            owner,
            "owner-keyed-latest-unkeyed",
        );

        let mut incompatible_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        incompatible_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        let mut incompatible_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut incompatible_runtime,
            &mut incompatible_keyed,
            7,
            owner,
            "owner-keyed-latest-incompatible",
        );
    }

    #[test]
    fn owner_keyed_latest_rejects_stale_owner_generation_without_fallback() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stale",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );

        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        runtime.refresh();
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let outcome = runtime.execute_command(command);

        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_host_rejection_restores_only_the_affected_predecessor() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        assert_owner_keyed_latest_rejected(
            &mut runtime,
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-host-rejected",
        );
    }

    #[test]
    fn owner_keyed_latest_capacity_rejection_restores_predecessor_without_fallback() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-keyed-latest-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-capacity-overflow",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&7), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_latest_public_execute_refreshes_before_removed_owner_resolution() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-removed-before-public-execute",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );

        runtime.bridge_mut().show_owner = false;
        let outcome = runtime.execute_command(command);

        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert!(outcome.surface_refresh_requested);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_latest_host_rejection_rolls_back_without_retry_or_mapping() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-host-rejected",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_latest_supersession_maps_only_the_current_ticket() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let first_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-first",
            move |completion| {
                first_state.borrow_mut().push(completion.ticket.id());
                completion.output
            },
        );
        let _ = runtime.execute_command(first_command);

        let second_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-second",
            move |completion| {
                second_state.borrow_mut().push(completion.ticket.id());
                completion.output
            },
        );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 2);
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![second_ticket.id()]);
    }

    #[test]
    fn owner_keyed_latest_supersession_fences_the_previous_keyed_ticket() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, usize)>::new()));

        let first_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-first",
            move |completion| {
                first_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                completion.output
            },
        );
        let _ = runtime.execute_command(first_command);

        let second_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket, second_key) =
            owner_keyed_latest_command(
                &mut keyed,
                key,
                owner,
                "owner-keyed-latest-second",
                move |completion| {
                    second_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    completion.output
                },
            );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(second_key, key);
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(keyed.active(&key), Some(second_ticket));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 2);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(key, second_ticket.id(), 7)]);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![7]
        );
    }

    #[test]
    fn owner_latest_retirement_suppresses_queued_completion_and_mapper() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, ticket) = owner_latest_command(
            &mut latest,
            owner,
            "owner-latest-retired",
            move |completion| {
                mapped_state.borrow_mut().push(completion.ticket.id());
                completion.output
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(latest.active(), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_retirement_suppresses_queued_completion_and_mapper() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-retired",
            move |completion| {
                mapped_state.borrow_mut().push(completion.ticket.id());
                completion.output
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_retirement_cancels_worker_and_fences_late_publication() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let cancellation_seen = Arc::new(AtomicBool::new(false));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let cancellation_seen_in_work = Arc::clone(&cancellation_seen);
        let mut keyed = KeyedLatestTasks::new();
        let (command, receipt, ticket, key) = owner_keyed_latest_command_with_work(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-retired-worker",
            move |worker_context| {
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut is_released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*is_released {
                    is_released = wake
                        .wait(is_released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                cancellation_seen_in_work.store(worker_context.is_cancelled(), Ordering::Release);
                7
            },
            move |completion| {
                mapped_state.borrow_mut().push(completion.output);
                completion.output
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred host captured keyed owner work");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("keyed owner worker completes");

        assert!(cancellation_seen.load(Ordering::Acquire));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_keyed_latest_preserves_owner_generation_across_compatible_reorder() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let before = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("initial keyed owner")
            .token
            .clone();
        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Reordered;
        runtime.refresh();
        let after = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == before.identity())
            .expect("reordered keyed owner")
            .token
            .clone();
        assert_eq!(after, before);
        assert_eq!(after.generation(), before.generation());
        assert!(runtime.declarative_owner_ledger().is_live(&after));

        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-reordered",
            move |completion| {
                mapped_state
                    .borrow_mut()
                    .push((completion.key, completion.ticket.id()));
                completion.output
            },
        );
        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(key, ticket.id())]);
    }

    #[test]
    fn owner_keyed_latest_closing_veto_closes_receipt_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_latest_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-closing",
            move |_| {
                mapped_state.borrow_mut().push(1);
                1
            },
        );

        let outcome = runtime.execute_command(crate::runtime::Command::batch(vec![
            crate::runtime::Command::exit(),
            command,
        ]));
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Closed
        );
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert!(outcome.exit_requested);
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_admission_preserves_fifo_key_ticket_sibling_and_reorder() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let owner_before = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("initial keyed owner")
            .token
            .clone();
        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Reordered;
        runtime.refresh();
        let owner_after = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == owner_before.identity())
            .expect("reordered keyed owner")
            .token
            .clone();
        assert_eq!(owner_after, owner_before);
        assert_eq!(owner_after.generation(), owner_before.generation());

        let mut keyed = KeyedLatestTasks::new();
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 3);
        assert_eq!(
            *mapped.borrow(),
            vec![
                (key, ticket.id(), 1),
                (key, ticket.id(), 2),
                (key, ticket.id(), 3),
            ]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1, 2, 3]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(
            *mapped.borrow(),
            vec![
                (key, ticket.id(), 1),
                (key, ticket.id(), 2),
                (key, ticket.id(), 3),
            ]
        );
    }

    fn assert_owner_keyed_ordered_stream_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        keyed: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let predecessor = keyed.begin(key);
        let sibling_key = key.wrapping_add(1);
        let sibling = keyed.begin(sibling_key);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt, replacement, request_key) = owner_keyed_ordered_stream_command(
            keyed,
            key,
            owner,
            name,
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(command);
        assert_eq!(request_key, key);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&sibling_key), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert!(runtime.worker_effects.registry.is_empty());
        assert!(runtime.worker_effects.pending_registrations.is_empty());
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            runtime
                .bridge()
                .reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    fn assert_owner_keyed_coalesced_stream_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        keyed: &mut KeyedLatestTasks<u8>,
        key: u8,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let predecessor = keyed.begin(key);
        let sibling_key = key.wrapping_add(1);
        let sibling = keyed.begin(sibling_key);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt, replacement, request_key) = owner_keyed_coalesced_stream_command(
            keyed,
            key,
            owner,
            name,
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(command);
        assert_eq!(request_key, key);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&sibling_key), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert!(runtime.worker_effects.registry.is_empty());
        assert!(runtime.worker_effects.pending_registrations.is_empty());
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            runtime
                .bridge()
                .reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_vetoes_without_fallback_and_rolls_back_only_affected_key()
    {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut invalid_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut invalid_runtime,
            &mut invalid_keyed,
            7,
            DeclarativeEffectOwner::new(),
            "owner-keyed-latest-stream-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        let mut removed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut removed_runtime,
            &mut removed_keyed,
            7,
            owner,
            "owner-keyed-latest-stream-removed",
        );

        let mut ambiguous_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        ambiguous_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Ambiguous;
        let mut ambiguous_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut ambiguous_runtime,
            &mut ambiguous_keyed,
            7,
            owner,
            "owner-keyed-latest-stream-ambiguous",
        );

        let mut unkeyed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        unkeyed_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Unkeyed;
        let mut unkeyed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut unkeyed_runtime,
            &mut unkeyed_keyed,
            7,
            owner,
            "owner-keyed-latest-stream-unkeyed",
        );

        let mut incompatible_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        incompatible_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        let mut incompatible_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut incompatible_runtime,
            &mut incompatible_keyed,
            7,
            owner,
            "owner-keyed-latest-stream-incompatible",
        );

        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut host_rejected_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_ordered_stream_rejected(
            &mut host_rejected_runtime,
            &mut host_rejected_keyed,
            7,
            owner,
            "owner-keyed-latest-stream-host-rejected",
        );
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_vetoes_without_fallback_and_rolls_back_only_affected_key()
     {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut invalid_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut invalid_runtime,
            &mut invalid_keyed,
            7,
            DeclarativeEffectOwner::new(),
            "owner-keyed-latest-coalesced-stream-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        let mut removed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut removed_runtime,
            &mut removed_keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-removed",
        );

        let mut ambiguous_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        ambiguous_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Ambiguous;
        let mut ambiguous_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut ambiguous_runtime,
            &mut ambiguous_keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-ambiguous",
        );

        let mut unkeyed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        unkeyed_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Unkeyed;
        let mut unkeyed_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut unkeyed_runtime,
            &mut unkeyed_keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-unkeyed",
        );

        let mut incompatible_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        incompatible_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        let mut incompatible_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut incompatible_runtime,
            &mut incompatible_keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-incompatible",
        );

        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut host_rejected_keyed = KeyedLatestTasks::new();
        assert_owner_keyed_coalesced_stream_rejected(
            &mut host_rejected_runtime,
            &mut host_rejected_keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-host-rejected",
        );
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_rejects_stale_owner_without_fallback() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-stale",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        runtime.refresh();
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let outcome = runtime.execute_command(command);

        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_capacity_rejection_restores_predecessor() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-keyed-latest-coalesced-stream-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-capacity-overflow",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert_eq!(keyed.active(&7), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_closing_veto_closes_receipt_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-closing",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(crate::runtime::Command::batch(vec![
            crate::runtime::Command::exit(),
            command,
        ]));
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Closed
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert!(outcome.exit_requested);
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_rejects_stale_owner_generation_without_fallback() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-stale",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        runtime.refresh();
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let outcome = runtime.execute_command(command);

        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_capacity_rejection_restores_predecessor_without_fallback()
    {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-keyed-latest-stream-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-capacity-overflow",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            runtime
                .bridge()
                .reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_supersession_fences_events_final_and_reducer() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));

        let first_event_state = Rc::clone(&mapped);
        let first_final_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-first",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                first_event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                first_final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );
        let _ = runtime.execute_command(first_command);

        let second_event_state = Rc::clone(&mapped);
        let second_final_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket, second_key) =
            owner_keyed_ordered_stream_command(
                &mut keyed,
                key,
                owner,
                "owner-keyed-latest-stream-second",
                |_, events| {
                    assert!(events.emit(4));
                    assert!(events.emit(5));
                    6
                },
                move |completion| {
                    second_event_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
                move |completion| {
                    second_final_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
            );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(second_key, key);
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(keyed.active(&key), Some(second_ticket));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 2);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 3);
        assert_eq!(
            *mapped.borrow(),
            vec![
                (key, second_ticket.id(), 4),
                (key, second_ticket.id(), 5),
                (key, second_ticket.id(), 6),
            ]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![4, 5, 6]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_incompatible_reinsertion_fences_old_generation() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let old_generation = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("initial owner generation")
            .token
            .clone();
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let first_event_state = Rc::clone(&mapped);
        let first_final_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-old-generation",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                first_event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                first_final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );
        let _ = runtime.execute_command(first_command);
        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(keyed.active(&key), Some(first_ticket));

        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        runtime.refresh();
        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Single;
        runtime.refresh();
        let reinserted_generation = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == old_generation.identity())
            .expect("reinserted owner generation")
            .token
            .clone();
        assert_ne!(reinserted_generation, old_generation);
        assert!(reinserted_generation.generation() > old_generation.generation());

        let second_event_state = Rc::clone(&mapped);
        let second_final_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket, second_key) =
            owner_keyed_ordered_stream_command(
                &mut keyed,
                key,
                owner,
                "owner-keyed-latest-stream-reinserted-generation",
                |_, events| {
                    assert!(events.emit(3));
                    4
                },
                move |completion| {
                    second_event_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
                move |completion| {
                    second_final_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
            );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(second_key, key);
        assert_ne!(second_ticket, first_ticket);
        assert_eq!(keyed.active(&key), Some(second_ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(key, second_ticket.id(), 3), (key, second_ticket.id(), 4),]
        );
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_retirement_suppresses_queued_events_final_and_reducer() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let reduced_messages = Arc::clone(&runtime.bridge().reduced_messages);
        let (command, receipt, ticket, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-retired",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();

        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_retirement_cancels_worker_and_fences_late_event_final() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let cancellation_seen = Arc::new(AtomicBool::new(false));
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let cancellation_seen_in_work = Arc::clone(&cancellation_seen);
        let mut keyed = KeyedLatestTasks::new();
        let (command, receipt, ticket, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-retired-worker",
            move |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut is_released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*is_released {
                    is_released = wake
                        .wait(is_released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                cancellation_seen_in_work.store(worker_context.is_cancelled(), Ordering::Release);
                assert!(!events.emit(2));
                3
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred host captured keyed owner stream work");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(key, ticket.id(), 1)]);
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread
            .join()
            .expect("keyed owner stream worker completes");

        assert!(cancellation_seen.load(Ordering::Acquire));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![(key, ticket.id(), 1)]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_closing_veto_closes_receipt_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement, key) = owner_keyed_ordered_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-stream-closing",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(crate::runtime::Command::batch(vec![
            crate::runtime::Command::exit(),
            command,
        ]));
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Closed
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_ne!(replacement, predecessor);
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert!(outcome.exit_requested);
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_keeps_newest_event_and_final_once() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let owner_before = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("initial keyed owner")
            .token
            .clone();
        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Reordered;
        runtime.refresh();
        let owner_after = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == owner_before.identity())
            .expect("reordered keyed owner")
            .token
            .clone();
        assert_eq!(owner_after, owner_before);
        assert_eq!(owner_after.generation(), owner_before.generation());

        let mut keyed = KeyedLatestTasks::new();
        let sibling = keyed.begin(8);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                for event in 1..=4 {
                    assert!(events.emit(event));
                }
                5
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(key, 7);
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(key, ticket.id(), 4), (key, ticket.id(), 5)]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![4, 5]
        );
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_coalesced,
            3
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(
            *mapped.borrow(),
            vec![(key, ticket.id(), 4), (key, ticket.id(), 5)]
        );
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_supersession_fences_late_event_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));

        let first_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket, key) =
            owner_keyed_coalesced_stream_command(
                &mut keyed,
                7,
                owner,
                "owner-keyed-latest-coalesced-stream-first",
                |_, events| {
                    assert!(events.emit(1));
                    assert!(events.emit(2));
                    3
                },
                move |completion| {
                    first_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
                move |completion| usize::from(completion.output),
            );
        let _ = runtime.execute_command(first_command);

        let second_event_state = Rc::clone(&mapped);
        let second_final_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket, second_key) =
            owner_keyed_coalesced_stream_command(
                &mut keyed,
                key,
                owner,
                "owner-keyed-latest-coalesced-stream-second",
                |_, events| {
                    assert!(events.emit(4));
                    assert!(events.emit(5));
                    6
                },
                move |completion| {
                    second_event_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
                move |completion| {
                    second_final_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
            );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(second_key, key);
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(keyed.active(&key), Some(second_ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(key, second_ticket.id(), 5), (key, second_ticket.id(), 6)]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![5, 6]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_event_supersession_fences_final_same_drain() {
        let owner = DeclarativeEffectOwner::new();
        let mut bridge = OwnerWorkerBridge::new(owner, true);
        bridge.keyed_supersession = Some(KeyedLatestTasks::new());
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let first_final_mapper_hits = Arc::new(AtomicUsize::new(0));
        let first_final_mapper_hits_for_mapper = Arc::clone(&first_final_mapper_hits);
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let first_event_state = Rc::clone(&mapped);
        let (command, receipt, first_ticket, key) = {
            let keyed = runtime
                .bridge_mut()
                .keyed_supersession
                .as_mut()
                .expect("keyed supersession registry");
            let mut context =
                crate::application::runtime::update_context::UiUpdateContext::<usize>::default();
            let request = context
                .business()
                .background("owner-keyed-latest-coalesced-stream-event-supersession")
                .latest_for(keyed, 7_u8);
            let first_ticket = request.ticket();
            let receipt = request.stream_latest_for_owner_with_receipt(
                owner,
                |_, events| {
                    assert!(events.emit(1_u8));
                    2_u8
                },
                move |completion| {
                    first_event_state.borrow_mut().push((
                        completion.key,
                        completion.ticket.id(),
                        completion.output,
                    ));
                    usize::from(completion.output)
                },
                move |completion| {
                    first_final_mapper_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                    usize::from(completion.output)
                },
            );
            (context.into_command(), receipt, first_ticket, 7_u8)
        };

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(key, 7);
        assert_eq!(
            runtime
                .bridge()
                .keyed_supersession
                .as_ref()
                .expect("keyed supersession registry")
                .active(&key),
            Some(first_ticket)
        );

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(key, first_ticket.id(), 1)]);
        assert_eq!(first_final_mapper_hits.load(Ordering::Acquire), 0);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1]
        );
        let second_ticket = runtime
            .bridge()
            .keyed_supersession
            .as_ref()
            .expect("event reducer should retain keyed supersession registry")
            .active(&key)
            .expect("event reducer should admit replacement keyed stream");
        assert_ne!(second_ticket, first_ticket);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![(key, first_ticket.id(), 1)]);
        assert_eq!(first_final_mapper_hits.load(Ordering::Acquire), 0);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1]
        );

        runtime.bridge_mut().run_deferred_worker();
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(*mapped.borrow(), vec![(key, first_ticket.id(), 1)]);
        assert_eq!(first_final_mapper_hits.load(Ordering::Acquire), 0);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1, 9, 10]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_retirement_suppresses_queued_event_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-retired",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();

        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_retirement_cancels_worker_before_work_runs() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let work_invoked = Arc::new(AtomicBool::new(false));
        let work_invoked_for_work = Arc::clone(&work_invoked);
        let mut keyed = KeyedLatestTasks::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u8, u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket, key) = owner_keyed_coalesced_stream_command(
            &mut keyed,
            7,
            owner,
            "owner-keyed-latest-coalesced-stream-retired-worker",
            move |worker_context, events| {
                work_invoked_for_work.store(true, Ordering::Release);
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push((
                    completion.key,
                    completion.ticket.id(),
                    completion.output,
                ));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred host captured keyed coalesced worker");
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        worker();

        assert!(!work_invoked.load(Ordering::Acquire));
        assert_eq!(keyed.active(&key), Some(ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_latest_ordered_stream_admission_preserves_fifo_and_exact_ticket() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket) = owner_latest_ordered_stream_command(
            &mut latest,
            owner,
            "owner-latest-stream-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(latest.active(), Some(ticket));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 3);
        assert_eq!(
            *mapped.borrow(),
            vec![(ticket.id(), 1), (ticket.id(), 2), (ticket.id(), 3)]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1, 2, 3]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(
            *mapped.borrow(),
            vec![(ticket.id(), 1), (ticket.id(), 2), (ticket.id(), 3)]
        );
    }

    #[test]
    fn owner_latest_coalesced_stream_keeps_newest_event_and_final_once() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                for event in 1..=4 {
                    assert!(events.emit(event));
                }
                5
            },
            move |completion| {
                event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(latest.active(), Some(ticket));
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(*mapped.borrow(), vec![(ticket.id(), 4), (ticket.id(), 5)]);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![4, 5]
        );
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_coalesced,
            3
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![(ticket.id(), 4), (ticket.id(), 5)]);
    }

    #[test]
    fn owner_latest_coalesced_stream_supersession_fences_late_event_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));

        let first_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-first",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |completion| {
                first_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| usize::from(completion.output),
        );
        let _ = runtime.execute_command(first_command);

        let second_event_state = Rc::clone(&mapped);
        let second_final_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-second",
            |_, events| {
                assert!(events.emit(4));
                assert!(events.emit(5));
                6
            },
            move |completion| {
                second_event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                second_final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(latest.active(), Some(second_ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(second_ticket.id(), 5), (second_ticket.id(), 6)]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![5, 6]
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_latest_coalesced_stream_retirement_cancels_work_and_fences_final() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let cancellation_seen = Arc::new(AtomicBool::new(false));
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let cancellation_seen_in_work = Arc::clone(&cancellation_seen);
        let mut latest = LatestTask::new();
        let (command, receipt, ticket) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-retired",
            move |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut is_released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*is_released {
                    is_released = wake
                        .wait(is_released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                cancellation_seen_in_work.store(worker_context.is_cancelled(), Ordering::Release);
                assert!(!events.emit(2));
                3
            },
            move |completion| {
                event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred host captured the owner latest worker");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(ticket.id(), 1)]);

        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(runtime.worker_effects.pending, 0);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("owner latest worker completes");

        assert!(cancellation_seen.load(Ordering::Acquire));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![(ticket.id(), 1)]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_latest_coalesced_stream_maps_separate_events_when_ui_drains_between_emissions() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let mut latest = LatestTask::new();
        let (command, receipt, ticket) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-drain-between",
            move |_, events| {
                assert!(events.emit(1));
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut is_released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*is_released {
                    is_released = wake
                        .wait(is_released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                assert!(events.emit(2));
                3
            },
            move |completion| {
                event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred host captured the owner latest worker");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![(ticket.id(), 1)]);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("owner latest worker completes");

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(ticket.id(), 1), (ticket.id(), 2), (ticket.id(), 3)]
        );
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_coalesced,
            0
        );
        assert_eq!(runtime.worker_effects.pending, 0);
    }

    fn assert_owner_latest_ordered_stream_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        latest: &mut LatestTask,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt, replacement) = owner_latest_ordered_stream_command(
            latest,
            owner,
            name,
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    fn assert_owner_latest_coalesced_stream_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        latest: &mut LatestTask,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt, replacement) = owner_latest_coalesced_stream_command(
            latest,
            owner,
            name,
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
        );

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_latest_coalesced_stream_rejections_restore_predecessor_without_spawn_or_mapping() {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut invalid_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut invalid_runtime,
            &mut invalid_latest,
            DeclarativeEffectOwner::new(),
            "owner-latest-coalesced-stream-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        let mut removed_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut removed_runtime,
            &mut removed_latest,
            owner,
            "owner-latest-coalesced-stream-removed",
        );

        let mut ambiguous_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        ambiguous_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Ambiguous;
        let mut ambiguous_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut ambiguous_runtime,
            &mut ambiguous_latest,
            owner,
            "owner-latest-coalesced-stream-ambiguous",
        );

        let mut unkeyed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        unkeyed_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Unkeyed;
        let mut unkeyed_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut unkeyed_runtime,
            &mut unkeyed_latest,
            owner,
            "owner-latest-coalesced-stream-unkeyed",
        );

        let mut incompatible_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        incompatible_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        let mut incompatible_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut incompatible_runtime,
            &mut incompatible_latest,
            owner,
            "owner-latest-coalesced-stream-incompatible",
        );

        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut host_rejected_latest = LatestTask::new();
        assert_owner_latest_coalesced_stream_rejected(
            &mut host_rejected_runtime,
            &mut host_rejected_latest,
            owner,
            "owner-latest-coalesced-stream-host-rejected",
        );
    }

    #[test]
    fn owner_latest_coalesced_stream_capacity_rejection_restores_predecessor() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-latest-coalesced-stream-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement) = owner_latest_coalesced_stream_command(
            &mut latest,
            owner,
            "owner-latest-coalesced-stream-capacity-overflow",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_latest_ordered_stream_rejections_restore_predecessor_without_spawn_or_mapping() {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut invalid_latest = LatestTask::new();
        assert_owner_latest_ordered_stream_rejected(
            &mut invalid_runtime,
            &mut invalid_latest,
            DeclarativeEffectOwner::new(),
            "owner-latest-stream-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        let mut removed_latest = LatestTask::new();
        assert_owner_latest_ordered_stream_rejected(
            &mut removed_runtime,
            &mut removed_latest,
            owner,
            "owner-latest-stream-removed",
        );

        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let mut host_rejected_latest = LatestTask::new();
        assert_owner_latest_ordered_stream_rejected(
            &mut host_rejected_runtime,
            &mut host_rejected_latest,
            owner,
            "owner-latest-stream-host-rejected",
        );
    }

    #[test]
    fn owner_latest_ordered_stream_capacity_rejection_restores_predecessor() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-latest-stream-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, replacement) = owner_latest_ordered_stream_command(
            &mut latest,
            owner,
            "owner-latest-stream-capacity-overflow",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
            move |completion| {
                final_state.borrow_mut().push(completion.output);
                usize::from(completion.output)
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(latest.active(), Some(predecessor));
        assert_ne!(replacement, predecessor);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_latest_ordered_stream_supersession_fences_late_events_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let bridge = OwnerWorkerBridge::new(owner, true);
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));

        let first_state = Rc::clone(&mapped);
        let (first_command, first_receipt, first_ticket) = owner_latest_ordered_stream_command(
            &mut latest,
            owner,
            "owner-latest-stream-first",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                first_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| usize::from(completion.output),
        );
        let _ = runtime.execute_command(first_command);

        let second_state = Rc::clone(&mapped);
        let (second_command, second_receipt, second_ticket) = owner_latest_ordered_stream_command(
            &mut latest,
            owner,
            "owner-latest-stream-second",
            |_, events| {
                assert!(events.emit(3));
                4
            },
            move |completion| {
                second_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            {
                let mapped = Rc::clone(&mapped);
                move |completion| {
                    mapped
                        .borrow_mut()
                        .push((completion.ticket.id(), completion.output));
                    usize::from(completion.output)
                }
            },
        );
        let _ = runtime.execute_command(second_command);

        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_ne!(first_ticket, second_ticket);
        assert_eq!(latest.active(), Some(second_ticket));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *mapped.borrow(),
            vec![(second_ticket.id(), 3), (second_ticket.id(), 4)]
        );
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![3, 4]
        );
    }

    #[test]
    fn owner_latest_ordered_stream_retirement_fences_late_events_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mut latest = LatestTask::new();
        let mapped = Rc::new(RefCell::new(Vec::<(u64, u8)>::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt, ticket) = owner_latest_ordered_stream_command(
            &mut latest,
            owner,
            "owner-latest-stream-retired",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |completion| {
                event_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
            move |completion| {
                final_state
                    .borrow_mut()
                    .push((completion.ticket.id(), completion.output));
                usize::from(completion.output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(latest.active(), Some(ticket));

        runtime.bridge_mut().show_owner = false;
        runtime.refresh();

        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
        assert!(
            runtime
                .bridge()
                .reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
    }

    #[test]
    fn owner_ordered_stream_admission_preserves_fifo_and_final_once() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(runtime.bridge().spawned.load(Ordering::Acquire), 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 3);
        assert_eq!(*mapped.borrow(), vec![1, 2, 3]);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![1, 2, 3]);
    }

    #[test]
    fn owner_ordered_stream_rejects_invalid_removed_and_host_rejected_owners() {
        let owner = DeclarativeEffectOwner::new();
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let (invalid_command, invalid_receipt) = owner_ordered_stream_command(
            DeclarativeEffectOwner::new(),
            "owner-stream-invalid",
            |_, _| 1,
            move |event| {
                mapped_state.borrow_mut().push(event);
                usize::from(event)
            },
            usize::from,
        );
        let _ = invalid_runtime.execute_command(invalid_command);
        assert_eq!(
            invalid_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(invalid_runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(mapped.borrow().is_empty());
        assert_eq!(
            invalid_runtime.drain_runtime_messages().messages_dispatched,
            0
        );

        let mapped_state = Rc::clone(&mapped);
        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let (removed_command, removed_receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-removed",
            |_, _| 1,
            move |event| {
                mapped_state.borrow_mut().push(event);
                usize::from(event)
            },
            usize::from,
        );
        removed_runtime.bridge_mut().show_owner = false;
        let outcome = removed_runtime.execute_command(removed_command);
        assert_eq!(
            removed_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(removed_runtime.bridge().spawned.load(Ordering::Acquire), 0);
        assert!(outcome.surface_refresh_requested);
        assert_eq!(
            removed_runtime.drain_runtime_messages().messages_dispatched,
            0
        );

        let mapped_state = Rc::clone(&mapped);
        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        let (host_command, host_receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-host-rejected",
            |_, _| 1,
            move |event| {
                mapped_state.borrow_mut().push(event);
                usize::from(event)
            },
            usize::from,
        );
        let _ = host_rejected_runtime.execute_command(host_command);
        assert_eq!(
            host_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(
            host_rejected_runtime
                .bridge()
                .spawned
                .load(Ordering::Acquire),
            0
        );
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_ordered_stream_retirement_suppresses_queued_events_final_and_mapper() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-retired",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();

        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_ordered_stream_retirement_cancels_context_and_rejects_later_emit() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let context_slot = Arc::new(Mutex::new(None));
        let sink_slot = Arc::new(Mutex::new(None));
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let work_context = Arc::clone(&context_slot);
        let work_sink = Arc::clone(&sink_slot);
        let (command, receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-probe",
            move |context, sink| {
                *work_context
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(context.clone());
                *work_sink
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(sink.clone());
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*released {
                    released = wake
                        .wait(released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                7
            },
            |_| 0,
            |_| 0,
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred worker host captured the task");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert!(
            context_slot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .as_ref()
                .is_some_and(|context| context.is_cancelled())
        );
        let sink = sink_slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("worker exposed its event sink");
        assert!(!sink.emit(9));

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("deferred worker completes");
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_coalesced_stream_keeps_newest_pending_event_and_final_once() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-valid",
            |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                for event in 1..=4 {
                    assert!(events.emit(event));
                }
                5
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(runtime.worker_effects.pending, 1);
        assert!(mapped.borrow().is_empty());

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(*mapped.borrow(), vec![4, 5]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.worker_effects.retained_completion_count(), 0);
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_coalesced,
            3
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![4, 5]);
    }

    #[test]
    fn owner_coalesced_stream_maps_separate_events_when_ui_drains_between_emissions() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-drain-between",
            move |_, events| {
                assert!(events.emit(1));
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*released {
                    released = wake
                        .wait(released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                assert!(events.emit(2));
                3
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred worker host captured the task");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![1]);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("deferred worker completes");

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(*mapped.borrow(), vec![1, 2, 3]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime.diagnostics.snapshot().queue.stream_events_coalesced,
            0
        );
    }

    #[test]
    fn owner_coalesced_stream_retirement_after_delivered_event_suppresses_later_event_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let pending_work = Arc::new(Mutex::new(None));
        let mut runtime = SurfaceRuntime::new(
            DeferredOwnerBridge {
                owner,
                show_owner: true,
                pending_work: Arc::clone(&pending_work),
            },
            Vector2::new(80.0, 40.0),
        );
        let ready = Arc::new((Mutex::new(false), Condvar::new()));
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let work_ready = Arc::clone(&ready);
        let work_released = Arc::clone(&released);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-retire-after-delivery",
            move |worker_context, events| {
                assert!(!worker_context.is_cancelled());
                assert!(events.emit(1));
                let (lock, wake) = &*work_ready;
                *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
                wake.notify_one();

                let (lock, wake) = &*work_released;
                let mut released = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                while !*released {
                    released = wake
                        .wait(released)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                }
                assert!(!events.emit(2));
                3
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        let worker = pending_work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .expect("deferred worker host captured the task");
        let worker_thread = std::thread::spawn(worker);

        let (lock, wake) = &*ready;
        let mut is_ready = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while !*is_ready {
            is_ready = wake
                .wait(is_ready)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(is_ready);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(*mapped.borrow(), vec![1]);

        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert_eq!(runtime.worker_effects.pending, 0);

        let (lock, wake) = &*released;
        *lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = true;
        wake.notify_one();
        worker_thread.join().expect("deferred worker completes");

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![1]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.worker_effects.retained_completion_count(), 0);
        assert!(runtime.worker_effects.registry.is_empty());
        assert!(runtime.worker_effects.pending_registrations.is_empty());
        assert_eq!(Rc::strong_count(&mapped), 1);
    }

    #[test]
    fn owner_coalesced_stream_retains_queued_messages_across_compatible_reorder() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-compatible-reorder",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert!(mapped.borrow().is_empty());
        let initial_owner = match &runtime
            .worker_effects
            .registry
            .values()
            .next()
            .expect("registered owner stream")
            .origin
        {
            EffectOrigin::Declarative(token) => token.clone(),
            _ => panic!("owner stream must be declarative-owned"),
        };

        runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Reordered;
        runtime.refresh();

        let current_owner = runtime
            .declarative_owner_ledger()
            .live_records()
            .iter()
            .find(|record| record.token.identity() == initial_owner.identity())
            .expect("compatible reordered owner remains live")
            .token
            .clone();
        assert_eq!(current_owner, initial_owner);
        assert_eq!(current_owner.generation(), initial_owner.generation());
        assert!(runtime.declarative_owner_ledger().is_live(&initial_owner));
        assert_eq!(runtime.worker_effects.pending, 1);

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(*mapped.borrow(), vec![1, 2]);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert_eq!(*mapped.borrow(), vec![1, 2]);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.worker_effects.retained_completion_count(), 0);
        assert_eq!(Rc::strong_count(&mapped), 1);
    }

    #[test]
    fn owner_coalesced_stream_retirement_suppresses_queued_event_and_final() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let event_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-retired",
            |_, events| {
                assert!(events.emit(1));
                assert!(events.emit(2));
                3
            },
            move |event| {
                event_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();

        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_coalesced_stream_event_retirement_fences_queued_final_mapper_and_reducer() {
        let owner = DeclarativeEffectOwner::new();
        let mut bridge = OwnerWorkerBridge::new(owner, true);
        bridge.retire_on_event = true;
        let final_reducer_hits = Arc::clone(&bridge.final_reducer_hits);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let final_mapper_hits = Arc::new(AtomicUsize::new(0));
        let final_mapper_hits_for_mapper = Arc::clone(&final_mapper_hits);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-event-retires-owner",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            usize::from,
            move |output| {
                final_mapper_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(final_mapper_hits.load(Ordering::Acquire), 0);
        assert_eq!(final_reducer_hits.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_coalesced_stream_same_owner_queued_before_drain_fences_later_mappers_and_reducers() {
        let owner = DeclarativeEffectOwner::new();
        let mut bridge = OwnerWorkerBridge::new(owner, true);
        bridge.retire_on_event = true;
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));

        let first_event_hits = Arc::new(AtomicUsize::new(0));
        let first_event_hits_for_mapper = Arc::clone(&first_event_hits);
        let first_final_hits = Arc::new(AtomicUsize::new(0));
        let first_final_hits_for_mapper = Arc::clone(&first_final_hits);
        let (first_command, first_receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-same-owner-first",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                first_event_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(event)
            },
            move |output| {
                first_final_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(output)
            },
        );

        let second_event_hits = Arc::new(AtomicUsize::new(0));
        let second_event_hits_for_mapper = Arc::clone(&second_event_hits);
        let second_final_hits = Arc::new(AtomicUsize::new(0));
        let second_final_hits_for_mapper = Arc::clone(&second_final_hits);
        let (second_command, second_receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-same-owner-second",
            |_, events| {
                assert!(events.emit(3));
                4
            },
            move |event| {
                second_event_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(event)
            },
            move |output| {
                second_final_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(first_command);
        let _ = runtime.execute_command(second_command);
        assert_eq!(
            first_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert_eq!(
            second_receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(first_event_hits.load(Ordering::Acquire), 1);
        assert_eq!(second_event_hits.load(Ordering::Acquire), 0);
        assert_eq!(first_final_hits.load(Ordering::Acquire), 0);
        assert_eq!(second_final_hits.load(Ordering::Acquire), 0);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1]
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    #[test]
    fn owner_coalesced_stream_event_closing_fences_later_same_drain_mapper() {
        let owner = DeclarativeEffectOwner::new();
        let mut bridge = OwnerWorkerBridge::new(owner, true);
        bridge.close_on_event = true;
        let reduced_messages = Arc::clone(&bridge.reduced_messages);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let event_hits = Arc::new(AtomicUsize::new(0));
        let event_hits_for_mapper = Arc::clone(&event_hits);
        let final_hits = Arc::new(AtomicUsize::new(0));
        let final_hits_for_mapper = Arc::clone(&final_hits);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-closing",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                event_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(event)
            },
            move |output| {
                final_hits_for_mapper.fetch_add(1, Ordering::AcqRel);
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(
            runtime.lifecycle_phase(),
            crate::runtime::RuntimeLifecyclePhase::Closing
        );
        assert_eq!(event_hits.load(Ordering::Acquire), 1);
        assert_eq!(final_hits.load(Ordering::Acquire), 0);
        assert_eq!(
            *reduced_messages
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1]
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert!(runtime.worker_effects.registry.is_empty());
    }

    fn assert_owner_coalesced_stream_rejected(
        runtime: &mut SurfaceRuntime<OwnerWorkerBridge, usize>,
        owner: DeclarativeEffectOwner,
        name: &'static str,
    ) {
        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            name,
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                mapped_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );

        let outcome = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(outcome.messages_dispatched, 0);
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn owner_coalesced_stream_rejects_invalid_surface_owner_variants() {
        let owner = DeclarativeEffectOwner::new();
        let mut invalid_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        assert_owner_coalesced_stream_rejected(
            &mut invalid_runtime,
            DeclarativeEffectOwner::new(),
            "owner-stream-latest-invalid",
        );

        let mut removed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        removed_runtime.bridge_mut().show_owner = false;
        assert_owner_coalesced_stream_rejected(
            &mut removed_runtime,
            owner,
            "owner-stream-latest-removed",
        );

        let mut ambiguous_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        ambiguous_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Ambiguous;
        assert_owner_coalesced_stream_rejected(
            &mut ambiguous_runtime,
            owner,
            "owner-stream-latest-ambiguous",
        );

        let mut unkeyed_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        unkeyed_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Unkeyed;
        assert_owner_coalesced_stream_rejected(
            &mut unkeyed_runtime,
            owner,
            "owner-stream-latest-unkeyed",
        );

        let mut incompatible_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        incompatible_runtime.bridge_mut().surface_mode = OwnerSurfaceMode::Incompatible;
        assert_owner_coalesced_stream_rejected(
            &mut incompatible_runtime,
            owner,
            "owner-stream-latest-incompatible",
        );

        let mut host_rejected_runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, false),
            Vector2::new(80.0, 40.0),
        );
        assert_owner_coalesced_stream_rejected(
            &mut host_rejected_runtime,
            owner,
            "owner-stream-latest-host-rejected",
        );
    }

    #[test]
    fn owner_coalesced_stream_rejects_same_update_removal_before_registration() {
        let owner = DeclarativeEffectOwner::new();
        let spawned = Arc::new(AtomicUsize::new(0));
        let mut runtime = SurfaceRuntime::new(
            SameUpdateRemovedOwnerBridge {
                owner,
                removed: false,
                receipt: None,
                spawned: Arc::clone(&spawned),
                latest: None,
                keyed_coalesced_stream: None,
                keyed_stream: None,
                keyed: None,
            },
            Vector2::new(80.0, 40.0),
        );

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(spawned.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime
                .bridge()
                .receipt
                .as_ref()
                .expect("same-update owner receipt")
                .poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_latest_coalesced_stream_rejects_same_update_removal_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let spawned = Arc::new(AtomicUsize::new(0));
        let mut latest = LatestTask::new();
        let predecessor = latest.begin();
        let mut runtime = SurfaceRuntime::new(
            SameUpdateRemovedOwnerBridge {
                owner,
                removed: false,
                receipt: None,
                spawned: Arc::clone(&spawned),
                latest: Some(latest),
                keyed_coalesced_stream: None,
                keyed_stream: None,
                keyed: None,
            },
            Vector2::new(80.0, 40.0),
        );

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(spawned.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime
                .bridge()
                .receipt
                .as_ref()
                .expect("same-update latest owner receipt")
                .poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(
            runtime
                .bridge()
                .latest
                .as_ref()
                .expect("same-update latest tracker")
                .active(),
            Some(predecessor)
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_rejects_same_update_removal_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let spawned = Arc::new(AtomicUsize::new(0));
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mut runtime = SurfaceRuntime::new(
            SameUpdateRemovedOwnerBridge {
                owner,
                removed: false,
                receipt: None,
                spawned: Arc::clone(&spawned),
                latest: None,
                keyed_coalesced_stream: None,
                keyed_stream: None,
                keyed: Some(keyed),
            },
            Vector2::new(80.0, 40.0),
        );

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(spawned.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime
                .bridge()
                .receipt
                .as_ref()
                .expect("same-update keyed owner receipt")
                .poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        let keyed = runtime
            .bridge()
            .keyed
            .as_ref()
            .expect("same-update keyed tracker");
        assert_eq!(keyed.active(&7), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_ordered_stream_rejects_same_update_removal_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let spawned = Arc::new(AtomicUsize::new(0));
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mut runtime = SurfaceRuntime::new(
            SameUpdateRemovedOwnerBridge {
                owner,
                removed: false,
                receipt: None,
                spawned: Arc::clone(&spawned),
                latest: None,
                keyed_coalesced_stream: None,
                keyed_stream: Some(keyed),
                keyed: None,
            },
            Vector2::new(80.0, 40.0),
        );

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(spawned.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime
                .bridge()
                .receipt
                .as_ref()
                .expect("same-update keyed stream owner receipt")
                .poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        let keyed = runtime
            .bridge()
            .keyed_stream
            .as_ref()
            .expect("same-update keyed stream tracker");
        assert_eq!(keyed.active(&7), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_keyed_latest_coalesced_stream_rejects_same_update_removal_and_rolls_back() {
        let owner = DeclarativeEffectOwner::new();
        let spawned = Arc::new(AtomicUsize::new(0));
        let mut keyed = KeyedLatestTasks::new();
        let predecessor = keyed.begin(7);
        let sibling = keyed.begin(8);
        let mut runtime = SurfaceRuntime::new(
            SameUpdateRemovedOwnerBridge {
                owner,
                removed: false,
                receipt: None,
                spawned: Arc::clone(&spawned),
                latest: None,
                keyed_coalesced_stream: Some(keyed),
                keyed_stream: None,
                keyed: None,
            },
            Vector2::new(80.0, 40.0),
        );

        let outcome = runtime.dispatch_message(0);
        assert_eq!(outcome.messages_dispatched, 1);
        assert_eq!(spawned.load(Ordering::Acquire), 0);
        assert_eq!(runtime.worker_effects.pending, 0);
        assert_eq!(
            runtime
                .bridge()
                .receipt
                .as_ref()
                .expect("same-update keyed coalesced stream owner receipt")
                .poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        let keyed = runtime
            .bridge()
            .keyed_coalesced_stream
            .as_ref()
            .expect("same-update keyed coalesced stream tracker");
        assert_eq!(keyed.active(&7), Some(predecessor));
        assert_eq!(keyed.active(&8), Some(sibling));
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 0);
    }

    #[test]
    fn owner_coalesced_stream_rejects_when_worker_capacity_is_saturated() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        for id in 0..EFFECT_INGRESS_CAPACITY {
            let command = crate::runtime::Command::perform_worker_effect_with_priority(
                "owner-stream-latest-capacity-fill",
                crate::runtime::TaskPriority::Background,
                None,
                0,
                move || id,
                |output| output,
            );
            let _ = runtime.execute_command(command);
        }

        let mapped = Rc::new(RefCell::new(Vec::new()));
        let mapped_state = Rc::clone(&mapped);
        let final_state = Rc::clone(&mapped);
        let (command, receipt) = owner_coalesced_stream_command(
            owner,
            "owner-stream-latest-capacity-overflow",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                mapped_state.borrow_mut().push(event);
                usize::from(event)
            },
            move |output| {
                final_state.borrow_mut().push(output);
                usize::from(output)
            },
        );
        let spawned_before = runtime.bridge().spawned.load(Ordering::Acquire);
        let outcome = runtime.execute_command(command);

        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Rejected
        );
        assert_eq!(spawned_before, EFFECT_INGRESS_CAPACITY);
        assert_eq!(
            runtime.bridge().spawned.load(Ordering::Acquire),
            spawned_before
        );
        assert_eq!(runtime.worker_effects.pending, EFFECT_INGRESS_CAPACITY);
        assert_eq!(outcome.messages_dispatched, 0);
        assert!(mapped.borrow().is_empty());
    }

    #[test]
    fn latest_and_owner_cancellation_probes_are_or_composed_for_work_context() {
        let owner = DeclarativeEffectOwner::new();
        let mut runtime = SurfaceRuntime::new(
            OwnerWorkerBridge::new(owner, true),
            Vector2::new(80.0, 40.0),
        );
        let token = runtime
            .declarative_owner_ledger()
            .live_records()
            .first()
            .expect("owner token")
            .token
            .clone();
        let origin = EffectOrigin::Declarative(token.clone());

        let mut latest = LatestTask::new();
        let transaction = latest.begin_replacement();
        let probe = combine_cancellation_probes(
            Some(transaction.cancellation_probe()),
            origin.cancellation_probe(),
        )
        .expect("composed owner/latest probe");
        assert!(!probe());
        let _superseding = latest.begin_replacement();
        assert!(probe(), "latest supersession must cancel work");

        let mut owner_latest = LatestTask::new();
        let owner_transaction = owner_latest.begin_replacement();
        let owner_probe = combine_cancellation_probes(
            Some(owner_transaction.cancellation_probe()),
            origin.cancellation_probe(),
        )
        .expect("composed owner/latest probe");
        assert!(!owner_probe());
        runtime.bridge_mut().show_owner = false;
        runtime.refresh();
        assert!(owner_probe(), "owner retirement must cancel work");
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
    fn application_ordered_and_latest_stream_mappers_are_eager_before_reducers() {
        for latest in [false, true] {
            let trace = Arc::new(Mutex::new(Vec::new()));
            let mut runtime = SurfaceRuntime::new(
                TraceBridge {
                    trace: Arc::clone(&trace),
                    close_on_event: false,
                },
                Vector2::new(80.0, 40.0),
            );
            let event_trace = Arc::clone(&trace);
            let final_trace = Arc::clone(&trace);
            runtime.execute_command(
                crate::runtime::Command::perform_worker_stream_with_priority(
                    "legacy-stream-eager",
                    crate::runtime::TaskPriority::Background,
                    crate::runtime::WorkerStreamOptions {
                        is_cancelled: None,
                        generation: 0,
                        latest,
                    },
                    move |sink| {
                        if latest {
                            assert!(sink.emit_latest(Box::new(1_u8)));
                        } else {
                            assert!(sink.emit(Box::new(1_u8)));
                        }
                        2_u8
                    },
                    move |event: u8| {
                        event_trace
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push("event");
                        usize::from(event)
                    },
                    move |output: u8| {
                        final_trace
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push("final");
                        usize::from(output)
                    },
                ),
            );

            assert!(
                trace
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .is_empty()
            );
            assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
            assert_eq!(
                *trace
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
                vec!["event", "final", "reduce", "reduce"]
            );
        }
    }

    #[test]
    fn legacy_stream_final_mapper_is_eager_before_shutdown_reducer() {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = SurfaceRuntime::new(
            TraceBridge {
                trace: Arc::clone(&trace),
                close_on_event: true,
            },
            Vector2::new(80.0, 40.0),
        );
        let event_trace = Arc::clone(&trace);
        let final_trace = Arc::clone(&trace);
        runtime.execute_command(
            crate::runtime::Command::perform_worker_stream_with_priority(
                "legacy-stream-eager-shutdown",
                crate::runtime::TaskPriority::Background,
                crate::runtime::WorkerStreamOptions {
                    is_cancelled: None,
                    generation: 0,
                    latest: false,
                },
                |sink| {
                    assert!(sink.emit(Box::new(1_u8)));
                    2_u8
                },
                move |event: u8| {
                    event_trace
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push("event");
                    usize::from(event)
                },
                move |output: u8| {
                    final_trace
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push("final");
                    usize::from(output)
                },
            ),
        );

        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 1);
        assert_eq!(
            runtime.lifecycle_phase(),
            crate::runtime::RuntimeLifecyclePhase::Closing
        );
        assert_eq!(
            *trace
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec!["event", "final", "reduce"]
        );
    }

    #[test]
    fn owner_ordered_stream_mappers_remain_eager_before_reducers() {
        let owner = DeclarativeEffectOwner::new();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let mut bridge = OwnerWorkerBridge::new(owner, true);
        bridge.trace = Some(Arc::clone(&trace));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(80.0, 40.0));
        let event_trace = Arc::clone(&trace);
        let final_trace = Arc::clone(&trace);
        let (command, receipt) = owner_ordered_stream_command(
            owner,
            "owner-stream-eager",
            |_, events| {
                assert!(events.emit(1));
                2
            },
            move |event| {
                event_trace
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push("event");
                usize::from(event)
            },
            move |output| {
                final_trace
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push("final");
                usize::from(output)
            },
        );

        let _ = runtime.execute_command(command);
        assert_eq!(
            receipt.poll(),
            crate::application::runtime::BusinessTaskAdmission::Accepted
        );
        assert!(
            trace
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .is_empty()
        );
        assert_eq!(runtime.drain_runtime_messages().messages_dispatched, 2);
        assert_eq!(
            *trace
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec!["event", "final", "reduce", "reduce"]
        );
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

    struct TraceBridge {
        trace: Arc<Mutex<Vec<&'static str>>>,
        close_on_event: bool,
    }

    impl crate::runtime::RuntimeBridge<usize> for TraceBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
                1,
                ContainerPolicy::default(),
                Vec::new(),
            )))
        }

        fn update(&mut self, message: usize) -> crate::runtime::Command<usize> {
            self.trace
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push("reduce");
            if self.close_on_event && message == 1 {
                return crate::runtime::Command::exit();
            }
            crate::runtime::Command::none()
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, usize> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<usize> for TraceBridge {
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
