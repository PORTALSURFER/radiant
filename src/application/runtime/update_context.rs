use crate::runtime::Command;

mod commands;
mod platform;
mod surface;
mod tasks;

/// Context supplied to app update closures for runtime-visible follow-up work.
pub struct UpdateContext<Message> {
    commands: Vec<Command<Message>>,
}

impl<Message> Default for UpdateContext<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<Message> UpdateContext<Message> {
    /// Consume this update context into the batched runtime command it queued.
    ///
    /// Most apps use Radiant's app builders, which collect this automatically.
    /// This method is for custom runtime bridges and tests that call a reducer
    /// directly but still need to execute queued runtime work.
    pub fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
