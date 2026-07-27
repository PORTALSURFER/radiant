//! UI-owned worker-effect completion routing.

use super::SurfaceRuntime;
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
    epoch: u64,
    result: EffectResult,
}

enum EffectResult {
    Event(Box<dyn Any + Send>),
    LatestEvent,
    Completed(Box<dyn Any + Send>),
    Cancelled,
    Panicked(String),
}

struct EffectIngress {
    sender: SyncSender<EffectTerminal>,
    sequence: Arc<Mutex<u64>>,
    finals: Arc<Mutex<VecDeque<EffectTerminal>>>,
    stream_events_coalesced: Arc<AtomicUsize>,
    stream_events_dropped: Arc<AtomicUsize>,
    stream_events_stale: Arc<AtomicUsize>,
}

impl EffectIngress {
    fn send(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        epoch: u64,
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
            epoch,
            result,
        };
        match self.sender.try_send(terminal) {
            Ok(()) => {
                *sequence = sequence.saturating_add(1);
                true
            }
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    fn send_event(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        epoch: u64,
        result: EffectResult,
    ) -> bool {
        let accepted = self.send(id, generation, epoch, result);
        if !accepted {
            self.stream_events_dropped.fetch_add(1, Ordering::AcqRel);
        }
        accepted
    }

    fn send_final(
        &self,
        id: EffectId,
        generation: EffectGeneration,
        epoch: u64,
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
            epoch,
            result,
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
    epoch: u64,
    is_cancelled: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    mapper: RegisteredMapper<Message>,
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
    epoch: u64,
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
        if self.ingress.send_event(
            self.id,
            self.generation,
            self.epoch,
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
    ingress: EffectIngress,
    receiver: std::sync::mpsc::Receiver<EffectTerminal>,
    deferred: VecDeque<EffectTerminal>,
    registry: HashMap<EffectId, Registered<Message>>,
    pending: usize,
    epoch: u64,
    stream_events_stale: usize,
}

impl<Message> Default for WorkerEffects<Message> {
    fn default() -> Self {
        let (ingress, receiver) = new_ingress();
        Self {
            ingress,
            receiver,
            deferred: VecDeque::new(),
            registry: HashMap::new(),
            pending: 0,
            epoch: 1,
            stream_events_stale: 0,
        }
    }
}

impl<Message> WorkerEffects<Message> {
    pub(super) fn submit<Bridge>(
        &mut self,
        runtime: &mut SurfaceRuntime<Bridge, Message>,
        effect: crate::runtime::command::WorkerEffect<Message>,
    ) -> bool
    where
        Bridge: RuntimeBridge<Message>,
    {
        if self.pending >= EFFECT_INGRESS_CAPACITY {
            if let Some(transaction) = effect.transaction {
                transaction.reject();
            }
            return false;
        }
        let transaction = effect.transaction;
        let id = effect.id;
        let generation = effect.generation;
        let epoch = self.epoch;
        let is_cancelled: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>> =
            effect.is_cancelled.map(Arc::from);
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
                epoch,
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
                epoch,
                is_cancelled: is_cancelled.clone(),
                mapper,
            },
        );
        self.pending += 1;

        let ingress = Arc::new(self.ingress.clone_handle());
        let work = effect.work;
        let final_ingress = Arc::clone(&ingress);
        let stream_sink = stream_latest.map(|latest| {
            if latest {
                if let Some(latest_state) = latest_state.as_ref().cloned() {
                    let ordered = ingress.clone_handle();
                    WorkerEffectSink::new_latest(
                        move |payload| {
                            ordered.send_event(id, generation, epoch, EffectResult::Event(payload))
                        },
                        {
                            let latest_state = Arc::clone(&latest_state);
                            move |payload| latest_state.emit(payload)
                        },
                        move || latest_state.close(),
                    )
                } else {
                    let ordered = Arc::clone(&ingress);
                    WorkerEffectSink::new_ordered(move |payload| {
                        ordered.send_event(id, generation, epoch, EffectResult::Event(payload))
                    })
                }
            } else {
                let ordered = Arc::clone(&ingress);
                WorkerEffectSink::new_ordered(move |payload| {
                    ordered.send_event(id, generation, epoch, EffectResult::Event(payload))
                })
            }
        });
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
                    let _ =
                        final_ingress.send_final(id, generation, epoch, EffectResult::Cancelled);
                    return;
                }
                let result = panic::catch_unwind(AssertUnwindSafe(|| match work {
                    WorkerEffectWork::Once(work) => work(),
                    WorkerEffectWork::Stream(work) => {
                        let sink = stream_sink.unwrap_or_else(|| {
                            let ordered = Arc::clone(&final_ingress);
                            WorkerEffectSink::new_ordered(move |payload| {
                                ordered.send_event(
                                    id,
                                    generation,
                                    epoch,
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
                let _ = final_ingress.send_final(id, generation, epoch, terminal);
            }),
        );
        if !accepted {
            self.pending = self.pending.saturating_sub(1);
            if let Some(previous) = previous {
                self.registry.insert(id, previous);
            } else {
                self.registry.remove(&id);
            }
            if let Some(transaction) = transaction {
                transaction.reject();
            }
        } else if let Some(transaction) = transaction {
            transaction.accept();
        }
        accepted
    }

    #[cfg(test)]
    pub(super) fn drain(&mut self) -> Vec<Message> {
        self.drain_at_high_water(self.ingress.high_water())
    }

    pub(super) fn drain_with_diagnostics_budget_at_high_water(
        &mut self,
        diagnostics: &crate::runtime::RuntimeDiagnosticsRecorder,
        budget: usize,
        high_water: u64,
    ) -> (Vec<Message>, bool, bool) {
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
    fn drain_at_high_water(&mut self, high_water: u64) -> Vec<Message> {
        self.drain_at_high_water_budget(high_water, usize::MAX).0
    }

    fn drain_at_high_water_budget(
        &mut self,
        high_water: u64,
        budget: usize,
    ) -> (Vec<Message>, bool, bool) {
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
        let mut messages = Vec::new();
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

    fn apply_terminal(&mut self, terminal: EffectTerminal, messages: &mut Vec<Message>) {
        if terminal.epoch != self.epoch {
            return;
        }
        if matches!(
            &terminal.result,
            EffectResult::Completed(_) | EffectResult::Cancelled | EffectResult::Panicked(_)
        ) {
            self.pending = self.pending.saturating_sub(1);
        }
        let current = self.registry.get(&terminal.id).is_some_and(|entry| {
            entry.generation == terminal.generation && entry.epoch == terminal.epoch
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
                    messages.push(message);
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
                    messages.push(message);
                }
            }
            EffectResult::Completed(output) => {
                let Some(entry) = self.registry.remove(&terminal.id) else {
                    return;
                };
                if entry.is_cancelled.as_ref().is_some_and(|probe| probe()) {
                    return;
                }
                match entry.mapper {
                    RegisteredMapper::Once(map) => {
                        if let Some(message) = map(output) {
                            messages.push(message);
                        }
                    }
                    RegisteredMapper::Stream { map_final, .. } => {
                        if let Some(message) = map_final(output) {
                            messages.push(message);
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
        self.deferred.clear();
        let (ingress, receiver) = new_ingress();
        self.ingress = ingress;
        self.receiver = receiver;
        self.pending = 0;
        self.stream_events_stale = 0;
    }
}

fn new_ingress() -> (EffectIngress, std::sync::mpsc::Receiver<EffectTerminal>) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(EFFECT_INGRESS_CAPACITY);
    let finals = Arc::new(Mutex::new(VecDeque::new()));
    (
        EffectIngress {
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
    pub(super) fn submit_worker_effect(
        &mut self,
        effect: crate::runtime::command::WorkerEffect<Message>,
    ) -> bool {
        let mut effects = std::mem::take(&mut self.worker_effects);
        let accepted = effects.submit(self, effect);
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
        RuntimeTaskHost, SurfaceNode, UiSurface,
    };
    use crate::{
        gui::types::{Point, Vector2},
        runtime::SurfaceRuntime,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::{cell::RefCell, rc::Rc};

    fn register(effect: &mut WorkerEffects<usize>, id: u64, generation: u64) {
        effect.registry.insert(
            EffectId(id),
            Registered {
                generation: EffectGeneration(generation),
                epoch: effect.epoch,
                is_cancelled: None,
                mapper: RegisteredMapper::Once(Box::new(|output| {
                    Some(*output.downcast::<usize>().expect("usize output"))
                })),
            },
        );
        effect.pending += 1;
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
    fn cancellation_and_panic_remove_mapper_without_invocation() {
        let mut effects = WorkerEffects::<usize>::default();
        let invoked = Arc::new(AtomicUsize::new(0));
        effects.registry.insert(
            EffectId(3),
            Registered {
                generation: EffectGeneration(1),
                epoch: effects.epoch,
                is_cancelled: None,
                mapper: RegisteredMapper::Once(Box::new({
                    let invoked = Arc::clone(&invoked);
                    move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        Some(1)
                    }
                })),
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
                epoch: effects.epoch,
                is_cancelled: {
                    let cancelled = Arc::clone(&cancelled);
                    Some(Arc::new(move || cancelled.load(Ordering::Acquire)))
                },
                mapper: RegisteredMapper::Once(Box::new({
                    let invoked = Arc::clone(&invoked);
                    move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        Some(1)
                    }
                })),
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
                epoch: effects.epoch,
                is_cancelled: None,
                mapper: RegisteredMapper::Once(Box::new(move |output| {
                    let _marker = &mapper_marker;
                    let output = *output.downcast::<usize>().expect("usize output");
                    mapper_state.borrow_mut().push(output);
                    Some(output + 1)
                })),
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
            let identity = runtime.platform_registry.register(Box::new(move |_| {
                mapped.borrow_mut().push(id);
                id
            }));
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
            epoch: old_epoch,
            result: EffectResult::Completed(Box::new(70_usize)),
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
