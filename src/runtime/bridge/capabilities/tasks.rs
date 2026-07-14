use crate::{
    gui::repaint::RepaintSignal,
    runtime::{BusinessMessageSink, TaskPriority},
};
use std::{sync::Arc, time::Duration};

/// Optional host capability for background work and repaint signaling.
pub trait RuntimeTaskHost<Message> {
    /// Install a repaint signal for host-owned background work.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}

    /// Queue a delayed host-defined message.
    fn schedule_message(&mut self, _delay: Duration, _message: Message) -> bool {
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
    pub schedule_message: fn(&mut Bridge, Duration, Message) -> bool,
    pub spawn_message_task: fn(
        &mut Bridge,
        &'static str,
        TaskPriority,
        CancellationProbe,
        MessageWork<Message>,
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
            schedule_message: Bridge::schedule_message,
            spawn_message_task: Bridge::spawn_message_task,
            spawn_streaming_message_task: Bridge::spawn_streaming_message_task,
            spawn_latest_streaming_message_task: Bridge::spawn_latest_streaming_message_task,
        }
    }
}
