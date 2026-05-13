use super::Command;

impl<Message> Command<Message> {
    /// Flatten immediate host-defined messages carried by this command.
    ///
    /// Runtime-visible effects such as delayed messages, background work, focus,
    /// repaint, and exit are intentionally not flattened. Execute the command
    /// through `SurfaceRuntime` when those effects must be preserved.
    pub fn into_messages(self) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.message_collection_hint());
        self.collect_messages(&mut messages);
        messages
    }

    fn message_collection_hint(&self) -> usize {
        match self {
            Self::Message(_) => 1,
            Self::Batch(commands) => commands.iter().map(Self::message_collection_hint).sum(),
            Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::Exit => 0,
        }
    }

    fn collect_messages(self, messages: &mut Vec<Message>) {
        match self {
            Self::Message(message) => messages.push(message),
            Self::Batch(commands) => {
                for command in commands {
                    command.collect_messages(messages);
                }
            }
            Self::None
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::Exit => {}
        }
    }
}
