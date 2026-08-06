use super::{batching, *};
use crate::runtime::{RuntimeQueueItem, RuntimeTimerOwner};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        if !self.lifecycle_accepts_work() {
            return CommandOutcome::default();
        }
        let mut outcome = CommandOutcome::default();
        let (command_budget, message_budget, mut completion_budget) = self.runtime_drain_budget();
        let worker_high_water = self.worker_effects.high_water();

        // Admit the previous-turn external singleton before any mapper runs.
        // It consumes one shared controller-completion slot, while a result
        // enqueued during mapping remains pending for the next turn.
        let external_completion = self.take_pending_external_drag_completion();
        completion_budget =
            completion_budget.saturating_sub(usize::from(external_completion.is_some()));

        // Freeze and remove the eligible platform prefix before invoking any
        // mapper. Older overflow is folded behind the remaining frozen queue,
        // so arrivals produced by a mapper cannot jump ahead of it.
        let (platform_results, platform_work_remaining) = {
            let mut pending = self
                .platform_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pending.take_budgeted_pending_batch(completion_budget)
        };
        completion_budget = completion_budget.saturating_sub(platform_results.len());

        // Native external-drag completion is admitted only on the next UI
        // turn. The identity fence also makes duplicate and superseded
        // completions harmless before the mapper is invoked.
        if let Some(pending) = external_completion {
            self.dispatch_message_inner((pending.on_completed)(pending.result), &mut outcome);
        }

        // Platform results were frozen above the mapper boundary. Mapping
        // happens before commands newly admitted below, so even a synchronous
        // host completion cannot re-enter execution.
        for delivery in platform_results {
            if let Some(mapped) = self.platform_registry.map_delivery(delivery) {
                self.dispatch_message_inner_with_origin(
                    mapped.message,
                    &mut outcome,
                    mapped.origin,
                );
            }
        }

        // Worker effects are admitted only when the higher-precedence platform
        // lane has no retained remainder. The ingress takes a start-of-turn
        // high-water snapshot, so an immediate worker completion is necessarily
        // deferred to the next drain pass.
        let worker_budget = if platform_work_remaining {
            0
        } else {
            completion_budget
        };
        let (worker_messages, deferred, later_turn) = self
            .worker_effects
            .drain_with_diagnostics_budget_at_high_water(
                &self.diagnostics,
                worker_budget,
                worker_high_water,
            );
        let worker_work_remaining = platform_work_remaining || deferred;
        let worker_later_turn = later_turn;
        for mapped in worker_messages {
            self.dispatch_message_inner_with_origin(mapped.message, &mut outcome, mapped.origin);
        }

        // Preserve the precedence fence: bridge commands/items are admitted
        // only after both controller-owned completion lanes are clear.
        if self.lifecycle_accepts_work() && !platform_work_remaining && !worker_work_remaining {
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
            while self.lifecycle_accepts_work() {
                let Some(command) = command_batch.pop() else {
                    break;
                };
                self.execute_command_inner(command, &mut outcome);
            }
            if self.lifecycle_accepts_work() {
                self.runtime_work.restore_command_batch(command_batch);
            }

            let mut item_batch = self.runtime_work.take_queue_item_batch();
            while self.lifecycle_accepts_work() {
                let Some(item) = item_batch.pop() else {
                    break;
                };
                match item {
                    RuntimeQueueItem::Message(message) => {
                        self.dispatch_message_inner(message, &mut outcome);
                    }
                    RuntimeQueueItem::Timer(wake)
                        if wake.owner == RuntimeTimerOwner::Application =>
                    {
                        if let Some(message) =
                            self.host_capabilities
                                .queues
                                .as_ref()
                                .and_then(|capability| {
                                    (capability.map_runtime_timer_wake)(&mut self.bridge, wake)
                                })
                        {
                            self.dispatch_message_inner(message, &mut outcome);
                        }
                    }
                    RuntimeQueueItem::Timer(wake) => {
                        if let Some(mapped) = self.timer_effects.map_wake(wake) {
                            self.dispatch_message_inner_with_origin(
                                mapped.message,
                                &mut outcome,
                                mapped.origin,
                            );
                        }
                    }
                    RuntimeQueueItem::Delivery(delivery) => {
                        match delivery.downcast::<crate::runtime::PlatformResultDelivery>() {
                            Ok(delivery) => {
                                if let Some(mapped) = self.platform_registry.map_delivery(delivery)
                                {
                                    self.dispatch_message_inner_with_origin(
                                        mapped.message,
                                        &mut outcome,
                                        mapped.origin,
                                    );
                                }
                            }
                            Err(delivery) => {
                                if let Some(message) = self
                                    .host_capabilities
                                    .queues
                                    .as_ref()
                                    .and_then(|capability| {
                                        (capability.map_runtime_queue_delivery)(
                                            &mut self.bridge,
                                            delivery,
                                        )
                                    })
                                {
                                    self.dispatch_message_inner(message, &mut outcome);
                                }
                            }
                        }
                    }
                }
            }
            if self.lifecycle_accepts_work() {
                self.runtime_work.restore_queue_item_batch(item_batch);
            }
        }

        let controller_work_remaining = platform_work_remaining || worker_work_remaining;
        let pending_controller_completions = {
            let platform_pending = self
                .platform_results
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .pending_len();
            platform_pending
                + self.worker_effects.retained_completion_count()
                + usize::from(self.interaction.drag.pending_external_completion.is_some())
        };
        self.diagnostics
            .record_controller_completion_depth(pending_controller_completions);
        if controller_work_remaining {
            self.diagnostics.record_controller_completion_deferral();
        }

        if controller_work_remaining
            || worker_later_turn
            || pending_controller_completions != 0
            || self.runtime_work.has_remaining_work()
        {
            outcome.runtime_work_remaining = true;
            outcome.repaint_requested = true;
            self.repaint_requested = true;
        }

        self.finish_command_outcome(outcome)
    }

    fn runtime_drain_budget(&self) -> (usize, usize, usize) {
        if self.interaction.pointer.capture.is_some()
            || self.interaction.layout_capture.is_some()
            || self.interaction.drag.session.is_some()
        {
            return (
                batching::INTERACTIVE_RUNTIME_COMMANDS_PER_DRAIN,
                batching::INTERACTIVE_RUNTIME_MESSAGES_PER_DRAIN,
                batching::INTERACTIVE_CONTROLLER_COMPLETIONS_PER_DRAIN,
            );
        }
        (
            batching::DEFAULT_RUNTIME_COMMANDS_PER_DRAIN,
            batching::DEFAULT_RUNTIME_MESSAGES_PER_DRAIN,
            batching::DEFAULT_CONTROLLER_COMPLETIONS_PER_DRAIN,
        )
    }
}
