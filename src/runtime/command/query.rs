use super::Command;

impl<Message> Command<Message> {
    /// Return whether this command performs no work.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::None => true,
            Self::Message(_)
            | Self::RequestRepaint
            | Self::RequestPaintOnly
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::Exit => false,
            Self::Batch(commands) => commands.iter().all(Self::is_empty),
        }
    }

    /// Return whether this command or any nested command requests repaint.
    pub fn requests_repaint(&self) -> bool {
        match self {
            Self::RequestRepaint | Self::RequestPaintOnly => true,
            Self::Batch(commands) => commands.iter().any(Self::requests_repaint),
            Self::None
            | Self::Message(_)
            | Self::After { .. }
            | Self::Perform { .. }
            | Self::Focus(_)
            | Self::Exit => false,
        }
    }
}
