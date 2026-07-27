use super::{batching, *};
use crate::runtime::{RuntimeQueueItem, RuntimeTimerOwner};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        let (command_budget, message_budget) = self.runtime_drain_budget();

        // Platform results are admitted on the previous turn's high-water
        // snapshot. Mapping happens before commands newly admitted below, so
        // even a synchronous host completion cannot re-enter execution.
        let platform_results = {
            let mut pending = self
                .platform_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pending.take_pending()
        };
        for delivery in platform_results {
            if let Some(message) = self.platform_registry.map_delivery(delivery) {
                self.dispatch_message_inner(message, &mut outcome);
            }
        }

        // Worker effects are mapped only on this UI-owned turn. The ingress
        // takes a start-of-turn high-water snapshot, so an immediate worker
        // completion is necessarily deferred to the next drain pass.
        for message in self
            .worker_effects
            .drain_with_diagnostics(&self.diagnostics)
        {
            self.dispatch_message_inner(message, &mut outcome);
        }

        self.runtime_work.drain_bridge_commands(
            &mut self.bridge,
            self.host_capabilities.queues.as_ref(),
            command_budget,
        );
        // Admit the bridge queue-item high-water snapshot before executing
        // commands. Items produced by those commands remain for the next pass.
        self.runtime_work.drain_bridge_queue_items(
            &mut self.bridge,
            self.host_capabilities.queues.as_ref(),
            message_budget,
        );
        let mut command_batch = self.runtime_work.take_command_batch();
        while let Some(command) = command_batch.pop() {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_work.restore_command_batch(command_batch);

        let mut item_batch = self.runtime_work.take_queue_item_batch();
        while let Some(item) = item_batch.pop() {
            let message = match item {
                RuntimeQueueItem::Message(message) => Some(message),
                RuntimeQueueItem::Timer(wake) if wake.owner == RuntimeTimerOwner::Application => {
                    self.host_capabilities
                        .queues
                        .as_ref()
                        .and_then(|capability| {
                            (capability.map_runtime_timer_wake)(&mut self.bridge, wake)
                        })
                }
                RuntimeQueueItem::Timer(wake) => self.timer_effects.map_wake(wake),
                RuntimeQueueItem::Delivery(delivery) => match delivery
                    .downcast::<crate::runtime::PlatformResultDelivery>(
                ) {
                    Ok(delivery) => self.platform_registry.map_delivery(delivery),
                    Err(delivery) => {
                        self.host_capabilities
                            .queues
                            .as_ref()
                            .and_then(|capability| {
                                (capability.map_runtime_queue_delivery)(&mut self.bridge, delivery)
                            })
                    }
                },
            };
            if let Some(message) = message {
                self.dispatch_message_inner(message, &mut outcome);
            }
        }
        self.runtime_work.restore_queue_item_batch(item_batch);

        if self.runtime_work.has_remaining_work() {
            outcome.runtime_work_remaining = true;
            outcome.repaint_requested = true;
            self.repaint_requested = true;
        }

        self.finish_command_outcome(outcome)
    }

    fn runtime_drain_budget(&self) -> (usize, usize) {
        if self.interaction.pointer.capture.is_some() || self.interaction.drag.session.is_some() {
            return (
                batching::INTERACTIVE_RUNTIME_COMMANDS_PER_DRAIN,
                batching::INTERACTIVE_RUNTIME_MESSAGES_PER_DRAIN,
            );
        }
        (
            batching::DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
            batching::DEFAULT_RUNTIME_MESSAGES_PER_DRAIN,
        )
    }
}
