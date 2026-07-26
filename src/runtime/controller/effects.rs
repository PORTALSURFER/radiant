//! UI-owned worker-effect completion routing.

use super::SurfaceRuntime;
use crate::runtime::RuntimeBridge;
use crate::runtime::command::{EffectGeneration, EffectId};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    panic::{self, AssertUnwindSafe},
    sync::{
        Arc, Mutex,
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
    Completed(Box<dyn Any + Send>),
    Cancelled,
    Panicked(String),
}

struct EffectIngress {
    sender: SyncSender<EffectTerminal>,
    sequence: Arc<Mutex<u64>>,
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

    fn high_water(&self) -> u64 {
        *self
            .sequence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

struct Registered<Message> {
    generation: EffectGeneration,
    epoch: u64,
    is_cancelled: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    map: Box<dyn FnOnce(Box<dyn Any + Send>) -> Message + 'static>,
}

pub(super) struct WorkerEffects<Message> {
    ingress: EffectIngress,
    receiver: std::sync::mpsc::Receiver<EffectTerminal>,
    deferred: VecDeque<EffectTerminal>,
    registry: HashMap<EffectId, Registered<Message>>,
    pending: usize,
    epoch: u64,
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
            return false;
        }
        let id = effect.id;
        let generation = effect.generation;
        let epoch = self.epoch;
        let is_cancelled: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>> =
            effect.is_cancelled.map(Arc::from);
        let previous = self.registry.insert(
            id,
            Registered {
                generation,
                epoch,
                is_cancelled: is_cancelled.clone(),
                map: effect.map,
            },
        );
        self.pending += 1;

        let ingress = Arc::new(EffectIngress {
            sender: self.ingress.sender.clone(),
            sequence: Arc::clone(&self.ingress.sequence),
        });
        let work = effect.work;
        let accepted = runtime.host_spawn_worker_task(
            effect.name,
            effect.priority,
            is_cancelled.as_ref().map(|probe| {
                let probe = Arc::clone(probe);
                Box::new(move || probe()) as Box<dyn Fn() -> bool + Send + Sync + 'static>
            }),
            Box::new(move || {
                if is_cancelled.as_ref().is_some_and(|probe| probe()) {
                    let _ = ingress.send(id, generation, epoch, EffectResult::Cancelled);
                    return;
                }
                let result = panic::catch_unwind(AssertUnwindSafe(work));
                let terminal = match result {
                    Ok(_output) if is_cancelled.as_ref().is_some_and(|probe| probe()) => {
                        EffectResult::Cancelled
                    }
                    Ok(output) => EffectResult::Completed(output),
                    Err(payload) => EffectResult::Panicked(panic_message(payload)),
                };
                let _ = ingress.send(id, generation, epoch, terminal);
            }),
        );
        if !accepted {
            self.pending = self.pending.saturating_sub(1);
            if let Some(previous) = previous {
                self.registry.insert(id, previous);
            } else {
                self.registry.remove(&id);
            }
        }
        accepted
    }

    pub(super) fn drain(&mut self) -> Vec<Message> {
        self.drain_at_high_water(self.ingress.high_water())
    }

    fn drain_at_high_water(&mut self, high_water: u64) -> Vec<Message> {
        let mut messages = Vec::new();
        while let Some(terminal) = self.deferred.pop_front() {
            if terminal.sequence >= high_water {
                self.deferred.push_front(terminal);
                break;
            }
            self.apply_terminal(terminal, &mut messages);
        }
        while let Ok(terminal) = self.receiver.try_recv() {
            if terminal.sequence >= high_water {
                self.deferred.push_back(terminal);
                break;
            }
            self.apply_terminal(terminal, &mut messages);
        }
        messages
    }

    fn apply_terminal(&mut self, terminal: EffectTerminal, messages: &mut Vec<Message>) {
        if terminal.epoch != self.epoch {
            return;
        }
        self.pending = self.pending.saturating_sub(1);
        let current = self.registry.get(&terminal.id).is_some_and(|entry| {
            entry.generation == terminal.generation && entry.epoch == terminal.epoch
        });
        if !current {
            return;
        }
        let Some(entry) = self.registry.remove(&terminal.id) else {
            return;
        };
        match terminal.result {
            EffectResult::Completed(_output)
                if entry.is_cancelled.as_ref().is_some_and(|probe| probe()) => {}
            EffectResult::Completed(output) => messages.push((entry.map)(output)),
            EffectResult::Panicked(message) => {
                tracing::error!(effect.id = terminal.id.0, %message, "Radiant worker effect panicked")
            }
            EffectResult::Cancelled => {}
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
    }
}

fn new_ingress() -> (EffectIngress, std::sync::mpsc::Receiver<EffectTerminal>) {
    let (sender, receiver) = std::sync::mpsc::sync_channel(EFFECT_INGRESS_CAPACITY);
    (
        EffectIngress {
            sender,
            sequence: Arc::new(Mutex::new(0)),
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
    use crate::runtime::{RuntimeHostCapabilities, RuntimeTaskHost, SurfaceNode, UiSurface};
    use crate::{gui::types::Vector2, runtime::SurfaceRuntime};
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
                map: Box::new(|output| *output.downcast::<usize>().expect("usize output")),
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
        assert!(effects.drain_at_high_water(high_water).is_empty());
        assert_eq!(effects.drain(), vec![7]);
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
                map: {
                    let invoked = Arc::clone(&invoked);
                    Box::new(move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        1
                    })
                },
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
                map: {
                    let invoked = Arc::clone(&invoked);
                    Box::new(move |_| {
                        invoked.fetch_add(1, Ordering::AcqRel);
                        1
                    })
                },
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
                map: Box::new(move |output| {
                    let _marker = &mapper_marker;
                    let output = *output.downcast::<usize>().expect("usize output");
                    mapper_state.borrow_mut().push(output);
                    output + 1
                }),
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

    struct AdmissionBridge {
        accepted: Arc<AtomicUsize>,
    }

    impl crate::runtime::RuntimeBridge<usize> for AdmissionBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<usize>> {
            Arc::new(UiSurface::new(SurfaceNode::container(
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
}
