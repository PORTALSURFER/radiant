use crate::runtime::Command;

mod business;
mod commands;
mod platform;
mod surface;

pub use business::{BusinessRuntime, BusinessWorkContext};

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
    pub(in crate::application) fn queue_command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Access Radiant's business-work submission API.
    ///
    /// Use this for host-owned IO, decoding, cache hydration, persistence,
    /// analysis, and other work that must not run on the UI/event/render path.
    pub fn business(&mut self) -> BusinessRuntime<'_, Message> {
        BusinessRuntime::new(self)
    }

    /// Consume this update context into the batched runtime command it queued.
    ///
    /// Most apps use Radiant's app builders, which collect this automatically.
    /// This method is for custom runtime bridges and tests that call a reducer
    /// directly but still need to execute queued runtime work.
    pub fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
