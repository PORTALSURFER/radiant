use super::{batching, *};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        let (command_budget, message_budget) = self.runtime_drain_budget();

        let timer_wakes = self
            .runtime_work
            .drain_bridge_timer_wakes(&mut self.bridge, self.host_capabilities.queues.as_ref());
        self.runtime_work.set_timer_work_remaining(false);
        self.timer_effects.enqueue(timer_wakes);
        for message in self.timer_effects.drain(message_budget) {
            self.dispatch_message_inner(message, &mut outcome);
        }
        self.runtime_work
            .set_timer_work_remaining(self.timer_effects.has_remaining_work());

        // Worker effects are mapped only on this UI-owned turn. The ingress
        // takes a start-of-turn high-water snapshot, so an immediate worker
        // completion is necessarily deferred to the next drain pass.
        for message in self.worker_effects.drain() {
            self.dispatch_message_inner(message, &mut outcome);
        }

        self.runtime_work.drain_bridge_commands(
            &mut self.bridge,
            self.host_capabilities.queues.as_ref(),
            command_budget,
        );
        let mut command_batch = self.runtime_work.take_command_batch();
        while let Some(command) = command_batch.pop() {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_work.restore_command_batch(command_batch);

        self.runtime_work.drain_bridge_messages(
            &mut self.bridge,
            self.host_capabilities.queues.as_ref(),
            message_budget,
        );
        let mut message_batch = self.runtime_work.take_message_batch();
        while let Some(message) = message_batch.pop() {
            self.dispatch_message_inner(message, &mut outcome);
        }
        self.runtime_work.restore_message_batch(message_batch);

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
