//! Runtime-owned command and message queues used by bounded drain passes.

use super::Command;
use crate::runtime::controller::commands::batching::{
    take_runtime_command_batch_into, take_runtime_message_batch_into,
};
use crate::runtime::{RuntimeQueueCapability, RuntimeQueueItem};
pub(super) struct RuntimeWorkQueues<Message> {
    commands: Vec<Command<Message>>,
    command_batch: Vec<Command<Message>>,
    command_pending: Vec<Command<Message>>,
    queue_items: Vec<RuntimeQueueItem<Message>>,
    queue_item_batch: Vec<RuntimeQueueItem<Message>>,
    bridge_queue_items_remaining: bool,
    timer_ingress_closed: bool,
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

    pub(super) fn drain_bridge_queue_items<Bridge>(
        &mut self,
        bridge: &mut Bridge,
        capability: Option<&RuntimeQueueCapability<Bridge, Message>>,
        budget: usize,
    ) {
        self.bridge_queue_items_remaining = capability.is_some_and(|capability| {
            (capability.drain_runtime_queue_item_batch_into)(bridge, &mut self.queue_items, budget)
        });
        if self.timer_ingress_closed {
            self.queue_items
                .retain(|item| !matches!(item, RuntimeQueueItem::Timer(_)));
        }
        take_runtime_message_batch_into(&mut self.queue_items, &mut self.queue_item_batch, budget);
    }

    pub(super) fn fence_timer_wakes(&mut self) {
        self.timer_ingress_closed = true;
        self.queue_items
            .retain(|item| !matches!(item, RuntimeQueueItem::Timer(_)));
        self.queue_item_batch
            .retain(|item| !matches!(item, RuntimeQueueItem::Timer(_)));
    }

    pub(super) fn fence_all(&mut self) {
        self.commands.clear();
        self.command_batch.clear();
        self.command_pending.clear();
        self.queue_items.clear();
        self.queue_item_batch.clear();
        self.bridge_queue_items_remaining = false;
        self.fence_timer_wakes();
    }

    pub(super) fn take_command_batch(&mut self) -> Vec<Command<Message>> {
        std::mem::take(&mut self.command_batch)
    }

    pub(super) fn restore_command_batch(&mut self, batch: Vec<Command<Message>>) {
        self.command_batch = batch;
    }

    pub(super) fn take_queue_item_batch(&mut self) -> Vec<RuntimeQueueItem<Message>> {
        std::mem::take(&mut self.queue_item_batch)
    }

    pub(super) fn restore_queue_item_batch(&mut self, batch: Vec<RuntimeQueueItem<Message>>) {
        self.queue_item_batch = batch;
    }

    pub(super) fn has_remaining_work(&self) -> bool {
        !self.commands.is_empty()
            || !self.queue_items.is_empty()
            || self.bridge_queue_items_remaining
    }
}

impl<Message> Default for RuntimeWorkQueues<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            command_batch: Vec::new(),
            command_pending: Vec::new(),
            queue_items: Vec::new(),
            queue_item_batch: Vec::new(),
            bridge_queue_items_remaining: false,
            timer_ingress_closed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeQueueHost, RuntimeTimerWake};

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
        queues.drain_bridge_queue_items(&mut host, Some(&capability), 8);
        let items = queues.take_queue_item_batch();

        assert!(matches!(
            items.as_slice(),
            [
                RuntimeQueueItem::Timer(first),
                RuntimeQueueItem::Timer(second),
                RuntimeQueueItem::Timer(third)
            ] if *first == RuntimeTimerWake::application(2, 0, 1)
                && *second == RuntimeTimerWake::controller(1, 0, 1)
                && *third == RuntimeTimerWake::application(1, 0, 1)
        ));
    }

    #[test]
    fn fenced_timer_queue_discards_late_host_wakes() {
        let mut host = Host {
            wakes: vec![RuntimeTimerWake::application(1, 0, 1)],
        };
        let mut queues = RuntimeWorkQueues::<u8>::default();
        queues.fence_timer_wakes();
        let capability = RuntimeQueueCapability::new();
        queues.drain_bridge_queue_items(&mut host, Some(&capability), 8);

        assert!(queues.take_queue_item_batch().is_empty());
        assert!(host.wakes.is_empty());
    }
}
