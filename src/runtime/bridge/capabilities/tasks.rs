use crate::{
    gui::repaint::RepaintSignal,
    runtime::{BusinessMessageSink, TaskPriority},
};
use std::{sync::Arc, time::Duration};

/// Opaque timer identity delivered from a host timer lane to the UI runtime.
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

    /// Schedule an opaque timer wake. The host must not construct or transport
    /// an application message while waiting for the timer.
    fn schedule_timer(&mut self, _delay: Duration, _wake: RuntimeTimerWake) -> bool {
        false
    }

    /// Spawn message-producing host work.
    fn spawn_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce() -> Message + Send + 'static>,
    ) -> bool {
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

    /// Spawn ordered streaming host work.
    fn spawn_streaming_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        false
    }

    /// Spawn coalescing streaming host work.
    fn spawn_latest_streaming_message_task(
        &mut self,
        _name: &'static str,
        _priority: TaskPriority,
        _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        _work: Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>,
    ) -> bool {
        false
    }
}

type CancellationProbe = Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>;
type MessageWork<Message> = Box<dyn FnOnce() -> Message + Send + 'static>;
type StreamingWork<Message> = Box<dyn FnOnce(BusinessMessageSink<Message>) + Send + 'static>;

pub(crate) struct RuntimeTaskCapability<Bridge, Message> {
    pub install_repaint_signal: fn(&mut Bridge, Arc<dyn RepaintSignal>),
    pub schedule_timer: fn(&mut Bridge, Duration, RuntimeTimerWake) -> bool,
    pub spawn_message_task: fn(
        &mut Bridge,
        &'static str,
        TaskPriority,
        CancellationProbe,
        MessageWork<Message>,
    ) -> bool,
    pub spawn_worker_task: fn(
        &mut Bridge,
        &'static str,
        TaskPriority,
        CancellationProbe,
        Box<dyn FnOnce() + Send + 'static>,
    ) -> bool,
    pub spawn_streaming_message_task: fn(
        &mut Bridge,
        &'static str,
        TaskPriority,
        CancellationProbe,
        StreamingWork<Message>,
    ) -> bool,
    pub spawn_latest_streaming_message_task: fn(
        &mut Bridge,
        &'static str,
        TaskPriority,
        CancellationProbe,
        StreamingWork<Message>,
    ) -> bool,
}

impl<Bridge, Message> RuntimeTaskCapability<Bridge, Message>
where
    Bridge: RuntimeTaskHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            install_repaint_signal: Bridge::install_repaint_signal,
            schedule_timer: Bridge::schedule_timer,
            spawn_message_task: Bridge::spawn_message_task,
            spawn_worker_task: Bridge::spawn_worker_task,
            spawn_streaming_message_task: Bridge::spawn_streaming_message_task,
            spawn_latest_streaming_message_task: Bridge::spawn_latest_streaming_message_task,
        }
    }
}
