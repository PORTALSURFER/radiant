use crate::runtime::Command;

/// Optional host capability for runtime-owned command and message queues.
pub trait RuntimeQueueHost<Message> {
    /// Drain commands delivered by app startup or bridge-owned work.
    fn take_runtime_commands(&mut self) -> Vec<Command<Message>> {
        Vec::new()
    }

    /// Drain commands into caller-owned scratch storage.
    fn drain_runtime_commands_into(&mut self, commands: &mut Vec<Command<Message>>) {
        commands.extend(self.take_runtime_commands());
    }

    /// Drain messages delivered by app tasks, timers, or subscriptions.
    fn take_runtime_messages(&mut self) -> Vec<Message> {
        Vec::new()
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
}

impl<Bridge, Message> RuntimeQueueCapability<Bridge, Message>
where
    Bridge: RuntimeQueueHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            drain_runtime_commands_into: Bridge::drain_runtime_commands_into,
            drain_runtime_message_batch_into: Bridge::drain_runtime_message_batch_into,
        }
    }
}
