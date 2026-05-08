//! Generic command values returned or queued by host-side runtime code.

use crate::widgets::WidgetId;
use std::{fmt, time::Duration};

/// Runtime-facing command produced by host application logic.
///
/// Radiant commands are intentionally small and domain-neutral. Hosts keep
/// ownership of IO, background work, and other side effects; this type only
/// represents values the generic runtime can understand directly.
#[derive(Default)]
pub enum Command<Message> {
    /// No follow-up work is required.
    #[default]
    None,
    /// Dispatch a host-defined message.
    Message(Message),
    /// Dispatch multiple commands in order.
    Batch(Vec<Command<Message>>),
    /// Request another redraw from the active runtime adapter.
    RequestRepaint,
    /// Request redraw without forcing declarative surface reprojection.
    RequestPaintOnly,
    /// Dispatch a host-defined message after a delay.
    After {
        /// Delay before the message is delivered.
        delay: Duration,
        /// Message to dispatch.
        message: Message,
    },
    /// Run host work on a background thread and dispatch the resulting message.
    Perform {
        /// Human-readable task name for diagnostics.
        name: &'static str,
        /// Background work lowered into a message-producing closure.
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    },
    /// Move keyboard focus to one widget.
    Focus(WidgetId),
    /// Request that the active runtime exits.
    Exit,
}

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

    /// Build a command that repaints without refreshing the declarative surface.
    pub const fn request_paint_only() -> Self {
        Self::RequestPaintOnly
    }

    /// Build a command that dispatches one message after the provided delay.
    pub const fn after(delay: Duration, message: Message) -> Self {
        Self::After { delay, message }
    }

    /// Build a command that runs work on a background thread and maps its result
    /// into a host message.
    pub fn perform<Output>(
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::Perform {
            name,
            work: Box::new(move || map(work())),
        }
    }

    /// Build a command that moves keyboard focus to one widget.
    pub const fn focus(widget_id: WidgetId) -> Self {
        Self::Focus(widget_id)
    }

    /// Build a command that asks the active runtime to exit.
    pub const fn exit() -> Self {
        Self::Exit
    }

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

    /// Flatten immediate host-defined messages carried by this command.
    ///
    /// Runtime-visible effects such as delayed messages, background work, focus,
    /// repaint, and exit are intentionally not flattened. Execute the command
    /// through `SurfaceRuntime` when those effects must be preserved.
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
