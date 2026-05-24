use super::{
    batching::{take_runtime_command_batch_into, take_runtime_message_batch_into},
    *,
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Dispatch any messages queued by bridge-owned runtime work.
    pub fn drain_runtime_messages(&mut self) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.bridge
            .drain_runtime_commands_into(&mut self.runtime_commands);
        let (command_budget, message_budget) = self.runtime_drain_budget();
        take_runtime_command_batch_into(
            &mut self.runtime_commands,
            &mut self.runtime_command_batch,
            command_budget,
        );
        let mut command_batch = std::mem::take(&mut self.runtime_command_batch);
        while let Some(command) = command_batch.pop() {
            self.execute_command_inner(command, &mut outcome);
        }
        self.runtime_command_batch = command_batch;

        self.bridge
            .drain_runtime_messages_into(&mut self.runtime_messages);
        take_runtime_message_batch_into(
            &mut self.runtime_messages,
            &mut self.runtime_message_batch,
            message_budget,
        );
        while let Some(message) = self.runtime_message_batch.pop() {
            self.dispatch_message_inner(message, &mut outcome);
        }

        if !self.runtime_commands.is_empty() || !self.runtime_messages.is_empty() {
            outcome.runtime_work_remaining = true;
            outcome.repaint_requested = true;
            self.repaint_requested = true;
        }

        self.finish_command_outcome(outcome)
    }

    fn runtime_drain_budget(&self) -> (usize, usize) {
        if self.pointer_capture.is_some() || self.drag_session.is_some() {
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
