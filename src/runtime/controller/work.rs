//! Runtime-owned command and message queues used by bounded drain passes.

use super::Command;
use crate::runtime::RuntimeQueueCapability;
use crate::runtime::RuntimeTimerWake;
use crate::runtime::controller::commands::batching::{
    take_runtime_command_batch_into, take_runtime_message_batch_into,
};
use std::collections::VecDeque;

pub(super) struct RuntimeWorkQueues<Message> {
    commands: Vec<Command<Message>>,
    command_batch: Vec<Command<Message>>,
    command_pending: Vec<Command<Message>>,
    messages: Vec<Message>,
    message_batch: Vec<Message>,
    bridge_messages_remaining: bool,
    timer_wakes: VecDeque<RuntimeTimerWake>,
    timer_work_remaining: bool,
}

impl<Message> RuntimeWorkQueues<Message> {
    pub(super) fn drain_bridge_commands<Bridge>(
        &mut self,
        bridge: &mut Bridge,
        capability: Option<&RuntimeQueueCapability<Bridge, Message>>,
        budget: usize,
    ) {
        if let Some(capability) = capability {
            (capability.drain_runtime_commands_into)(bridge, &mut self.commands);
        }
        take_runtime_command_batch_into(
            &mut self.commands,
            &mut self.command_batch,
            &mut self.command_pending,
            budget,
        );
    }

    pub(super) fn drain_bridge_messages<Bridge>(
        &mut self,
        bridge: &mut Bridge,
        capability: Option<&RuntimeQueueCapability<Bridge, Message>>,
        budget: usize,
    ) {
        self.bridge_messages_remaining = capability.is_some_and(|capability| {
            (capability.drain_runtime_message_batch_into)(bridge, &mut self.messages, budget)
        });
        take_runtime_message_batch_into(&mut self.messages, &mut self.message_batch, budget);
    }

    pub(super) fn drain_bridge_timer_wakes<Bridge>(
        &mut self,
        bridge: &mut Bridge,
        capability: Option<&RuntimeQueueCapability<Bridge, Message>>,
    ) {
        if let Some(capability) = capability {
            self.timer_wakes
                .extend((capability.take_runtime_timer_wakes)(bridge));
        }
    }

    pub(super) fn pop_timer_wake(&mut self) -> Option<RuntimeTimerWake> {
        self.timer_wakes.pop_front()
    }

    pub(super) fn timer_wake_len(&self) -> usize {
        self.timer_wakes.len()
    }

    pub(super) fn clear_timer_wakes(&mut self) {
        self.timer_wakes.clear();
    }

    pub(super) fn take_command_batch(&mut self) -> Vec<Command<Message>> {
        std::mem::take(&mut self.command_batch)
    }

    pub(super) fn restore_command_batch(&mut self, batch: Vec<Command<Message>>) {
        self.command_batch = batch;
    }

    pub(super) fn take_message_batch(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.message_batch)
    }

    pub(super) fn restore_message_batch(&mut self, batch: Vec<Message>) {
        self.message_batch = batch;
    }

    pub(super) fn has_remaining_work(&self) -> bool {
        self.timer_work_remaining
            || !self.commands.is_empty()
            || !self.messages.is_empty()
            || self.bridge_messages_remaining
    }

    pub(super) fn set_timer_work_remaining(&mut self, remaining: bool) {
        self.timer_work_remaining = remaining;
    }
}

impl<Message> Default for RuntimeWorkQueues<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            command_batch: Vec::new(),
            command_pending: Vec::new(),
            messages: Vec::new(),
            message_batch: Vec::new(),
            bridge_messages_remaining: false,
            timer_wakes: VecDeque::new(),
            timer_work_remaining: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeQueueHost;

    #[derive(Default)]
    struct Host {
        wakes: Vec<RuntimeTimerWake>,
    }

    impl RuntimeQueueHost<u8> for Host {
        fn take_runtime_timer_wakes(&mut self) -> Vec<RuntimeTimerWake> {
            std::mem::take(&mut self.wakes)
        }
    }

    #[test]
    fn unified_timer_queue_preserves_cross_owner_fifo_order() {
        let mut host = Host {
            wakes: vec![
                RuntimeTimerWake::application(1, 0, 1),
                RuntimeTimerWake::controller(1, 0, 1),
                RuntimeTimerWake::application(2, 0, 1),
            ],
        };
        let mut queues = RuntimeWorkQueues::<u8>::default();
        let capability = RuntimeQueueCapability::new();
        queues.drain_bridge_timer_wakes(&mut host, Some(&capability));

        assert_eq!(queues.timer_wake_len(), 3);
        assert_eq!(
            queues.pop_timer_wake(),
            Some(RuntimeTimerWake::application(1, 0, 1))
        );
        assert_eq!(
            queues.pop_timer_wake(),
            Some(RuntimeTimerWake::controller(1, 0, 1))
        );
        assert_eq!(
            queues.pop_timer_wake(),
            Some(RuntimeTimerWake::application(2, 0, 1))
        );
        assert_eq!(queues.pop_timer_wake(), None);
    }
}
