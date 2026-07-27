use crate::{gui::repaint::RepaintSignal, runtime::TaskPriority};
use std::{sync::Arc, time::Duration};

/// Opaque timer identity delivered from a host timer lane to the UI runtime.
///
/// Timer threads and custom hosts carry this value only; they never construct,
/// transport, or reduce an application message. The UI runtime owns wake
/// ordering, generation/epoch validation, mapper invocation, and message
/// reduction. See [`RuntimeTaskHost::schedule_timer`],
/// [`crate::runtime::RuntimeQueueHost::take_runtime_timer_wakes`], and
/// [`crate::runtime::RuntimeQueueHost::map_runtime_timer_wake`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeTimerWake {
    pub(crate) id: u64,
    pub(crate) generation: u64,
    pub(crate) epoch: u64,
    pub(crate) owner: RuntimeTimerOwner,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Owner namespace for an opaque timer wake identity.
pub enum RuntimeTimerOwner {
    /// Wake belongs to the application-owned UI timer registry.
    Application,
    /// Wake belongs to the controller-owned deferred-effect registry.
    Controller,
}

impl RuntimeTimerWake {
    pub(crate) const fn new(
        id: u64,
        generation: u64,
        epoch: u64,
        owner: RuntimeTimerOwner,
    ) -> Self {
        Self {
            id,
            generation,
            epoch,
            owner,
        }
    }

    pub(crate) const fn application(id: u64, generation: u64, epoch: u64) -> Self {
        Self::new(id, generation, epoch, RuntimeTimerOwner::Application)
    }

    pub(crate) const fn controller(id: u64, generation: u64, epoch: u64) -> Self {
        Self::new(id, generation, epoch, RuntimeTimerOwner::Controller)
    }
}

/// Optional host capability for background work and repaint signaling.
pub trait RuntimeTaskHost<Message> {
    /// Install a repaint signal for host-owned background work.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}

    /// Schedule an opaque timer wake on the host timer lane.
    ///
    /// The host carries only `wake` while waiting: it must not construct,
    /// transport, map, or reduce an application message on the timer thread.
    /// The UI runtime later receives the wake through
    /// [`crate::runtime::RuntimeQueueHost::take_runtime_timer_wakes`], validates it, and owns
    /// mapper invocation and message reduction. Return `true` when the wake was
    /// accepted by the host timer lane.
    fn schedule_timer(&mut self, _delay: Duration, _wake: RuntimeTimerWake) -> bool {
        false
    }

    /// Spawn worker-only work that reports completion through a runtime-owned
    /// ingress. The closure must not construct or transport an application
    /// message.
    fn spawn_worker_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce() + Send + 'static>,
    ) -> bool {
        false
    }
}

type CancellationProbe = Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>;
type WorkerTask = Box<dyn FnOnce() + Send + 'static>;
type SpawnWorkerTask<Bridge> =
    fn(&mut Bridge, &'static str, TaskPriority, CancellationProbe, WorkerTask) -> bool;

pub(crate) struct RuntimeTaskCapability<Bridge, Message> {
    pub install_repaint_signal: fn(&mut Bridge, Arc<dyn RepaintSignal>),
    pub schedule_timer: fn(&mut Bridge, Duration, RuntimeTimerWake) -> bool,
    pub spawn_worker_task: SpawnWorkerTask<Bridge>,
    pub(super) _message: std::marker::PhantomData<fn() -> Message>,
}

impl<Bridge, Message> RuntimeTaskCapability<Bridge, Message>
where
    Bridge: RuntimeTaskHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            install_repaint_signal: Bridge::install_repaint_signal,
            schedule_timer: Bridge::schedule_timer,
            spawn_worker_task: Bridge::spawn_worker_task,
            _message: std::marker::PhantomData,
        }
    }
}
