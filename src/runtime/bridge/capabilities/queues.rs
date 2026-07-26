use crate::runtime::{Command, RuntimeTimerWake};

/// Optional host capability for runtime-owned command, message, and timer-wake
/// queues.
///
/// A custom host exposes timer completion as [`RuntimeTimerWake`] values, not
/// application messages. The UI runtime drains those wakes, validates their
/// owner and generation, invokes the application mapper, and reduces any
/// resulting message on the UI owner. No timer-thread message path exists.
pub trait RuntimeQueueHost<Message> {
    /// Drain commands delivered by app startup or bridge-owned work.
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        Vec::new()
    }

    /// Drain commands into caller-owned scratch storage.
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        commands.extend(self.take_runtime_commands());
    }

    /// Drain messages delivered by app tasks or worker subscriptions.
    ///
    /// Timer completions use [`Self::take_runtime_timer_wakes`] and are mapped
    /// on the UI turn instead of arriving here as ordinary timer messages.
    fn take_runtime_messages(&mut self) -> Vec<Message> {
        Vec::new()
    }

    /// Drain opaque timer wakes delivered by a host timer lane.
    ///
    /// Custom hosts must implement this ingress for delayed commands and
    /// interval subscriptions. The timer lane carries only the wake; the UI
    /// runtime owns FIFO ordering, generation/epoch validation, mapper
    /// invocation, and message reduction. Omitting this ingress drops timer
    /// work before the UI controller can map or repaint it.
    fn take_runtime_timer_wakes(&mut self) -> Vec<RuntimeTimerWake> {
        Vec::new()
    }

    /// Map an application-owned timer wake on the UI turn.
    ///
    /// The runtime calls this only when the wake reaches the unified FIFO head.
    /// It owns generation/epoch validation and invokes this mapper on the UI
    /// owner; controller-owned wakes are mapped by the runtime controller.
    /// A host must not invoke this method from its timer thread.
    fn map_runtime_timer_wake(&mut self, _wake: RuntimeTimerWake) -> Option<Message> {
        None
    }

    /// Drain messages into caller-owned scratch storage.
    fn drain_runtime_messages_into(&mut self, messages: &mut Vec<Message>) {
        messages.extend(self.take_runtime_messages());
    }

    /// Drain one bounded controller pass and report whether more remain.
    fn drain_runtime_message_batch_into(
        &mut self,
        messages: &mut Vec<Message>,
        _max_messages: usize,
    ) -> bool {
        self.drain_runtime_messages_into(messages);
        false
    }
}

pub(crate) struct RuntimeQueueCapability<Bridge, Message> {
    pub drain_runtime_commands_into: fn(&mut Bridge, &mut Vec<Command<Message>>),
    pub drain_runtime_message_batch_into: fn(&mut Bridge, &mut Vec<Message>, usize) -> bool,
    pub take_runtime_timer_wakes: fn(&mut Bridge) -> Vec<RuntimeTimerWake>,
    pub map_runtime_timer_wake: fn(&mut Bridge, RuntimeTimerWake) -> Option<Message>,
}

impl<Bridge, Message> RuntimeQueueCapability<Bridge, Message>
where
    Bridge: RuntimeQueueHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            drain_runtime_commands_into: Bridge::drain_runtime_commands_into,
            drain_runtime_message_batch_into: Bridge::drain_runtime_message_batch_into,
            take_runtime_timer_wakes: Bridge::take_runtime_timer_wakes,
            map_runtime_timer_wake: Bridge::map_runtime_timer_wake,
        }
    }
}
