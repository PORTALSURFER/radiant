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
    pub(super) fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
