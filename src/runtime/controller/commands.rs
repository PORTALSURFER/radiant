use super::*;

pub(super) mod batching;
mod dispatch;
mod drain;
mod external_drag;
mod outcome;
mod scrolling;

pub use outcome::CommandOutcome;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.execute_command_inner(command, &mut outcome);
        self.finish_command_outcome(outcome)
    }
}

#[cfg(test)]
mod tests;
