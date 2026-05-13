use super::Command;
use std::fmt;

impl<Message> fmt::Debug for Command<Message>
where
    Message: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Message(message) => f.debug_tuple("Message").field(message).finish(),
            Self::Batch(commands) => f.debug_tuple("Batch").field(commands).finish(),
            Self::RequestRepaint => f.write_str("RequestRepaint"),
            Self::RequestPaintOnly => f.write_str("RequestPaintOnly"),
            Self::After { delay, message } => f
                .debug_struct("After")
                .field("delay", delay)
                .field("message", message)
                .finish(),
            Self::Perform { name, .. } => f.debug_struct("Perform").field("name", name).finish(),
            Self::Focus(widget_id) => f.debug_tuple("Focus").field(widget_id).finish(),
            Self::Exit => f.write_str("Exit"),
        }
    }
}
