//! Generic command values returned or queued by host-side runtime code.

/// Runtime-facing command produced by host application logic.
///
/// Radiant commands are intentionally small and domain-neutral. Hosts keep
/// ownership of IO, background work, and other side effects; this type only
/// represents values the generic runtime can understand directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command<Message> {
    /// No follow-up work is required.
    None,
    /// Dispatch a host-defined message.
    Message(Message),
    /// Dispatch multiple commands in order.
    Batch(Vec<Command<Message>>),
    /// Request another redraw from the active runtime adapter.
    RequestRepaint,
}

impl<Message> Default for Command<Message> {
    fn default() -> Self {
        Self::None
    }
}

impl<Message> Command<Message> {
    /// Return an empty command.
    pub const fn none() -> Self {
        Self::None
    }

    /// Build a command that dispatches one host-defined message.
    pub const fn message(message: Message) -> Self {
        Self::Message(message)
    }

    /// Build a command that dispatches multiple commands in order.
    pub fn batch(commands: impl IntoIterator<Item = Command<Message>>) -> Self {
        let commands = commands
            .into_iter()
            .filter(|command| !command.is_empty())
            .collect::<Vec<_>>();
        if commands.is_empty() {
            Self::None
        } else {
            Self::Batch(commands)
        }
    }

    /// Build a command that asks the active runtime adapter to repaint.
    pub const fn request_repaint() -> Self {
        Self::RequestRepaint
    }

    /// Return whether this command performs no work.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Message(_) | Self::RequestRepaint => false,
            Self::Batch(commands) => commands.iter().all(Self::is_empty),
        }
    }

    /// Return whether this command or any nested command requests repaint.
    pub fn requests_repaint(&self) -> bool {
        match self {
            Self::RequestRepaint => true,
            Self::Batch(commands) => commands.iter().any(Self::requests_repaint),
            Self::None | Self::Message(_) => false,
        }
    }

    /// Flatten all host-defined messages carried by this command.
    pub fn into_messages(self) -> Vec<Message> {
        let mut messages = Vec::new();
        self.collect_messages(&mut messages);
        messages
    }

    fn collect_messages(self, messages: &mut Vec<Message>) {
        match self {
            Self::Message(message) => messages.push(message),
            Self::Batch(commands) => {
                for command in commands {
                    command.collect_messages(messages);
                }
            }
            Self::None | Self::RequestRepaint => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn batch_drops_empty_commands_and_preserves_message_order() {
        let command = Command::batch([
            Command::none(),
            Command::message("first"),
            Command::batch([Command::message("second")]),
        ]);

        assert_eq!(command.into_messages(), vec!["first", "second"]);
    }

    #[test]
    fn repaint_requests_are_detected_through_nested_batches() {
        let command = Command::<()>::batch([
            Command::none(),
            Command::batch([Command::request_repaint()]),
        ]);

        assert!(command.requests_repaint());
    }
}
