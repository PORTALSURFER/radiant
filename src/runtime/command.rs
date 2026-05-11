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
    pub fn batch(command_iter: impl IntoIterator<Item = Command<Message>>) -> Self {
        let command_iter = command_iter.into_iter();
        let mut commands = Vec::with_capacity(command_iter.size_hint().0);
        for command in command_iter {
            command.append_to_batch(&mut commands);
        }
        match commands.len() {
            0 => Self::None,
            1 => commands.pop().expect("single command exists"),
            _ => Self::Batch(commands),
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

    fn append_to_batch(self, commands: &mut Vec<Command<Message>>) {
        match self {
            Self::None => {}
            Self::Batch(nested) => {
                commands.reserve(nested.len());
                for command in nested {
                    command.append_to_batch(commands);
                }
            }
            command => commands.push(command),
        }
    }

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
            Self::Batch(commands) => commands.len(),
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
                messages.reserve(commands.len());
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
    fn batch_flattens_nested_command_groups() {
        let command = Command::batch([
            Command::batch([Command::message("first")]),
            Command::batch([
                Command::none(),
                Command::batch([Command::message("second")]),
            ]),
        ]);

        let Command::Batch(commands) = &command else {
            panic!("non-empty nested batch should stay a batch");
        };

        assert_eq!(commands.len(), 2);
        assert!(matches!(commands[0], Command::Message("first")));
        assert!(matches!(commands[1], Command::Message("second")));
        assert_eq!(command.into_messages(), vec!["first", "second"]);
    }

    #[test]
    fn message_flattening_preserves_nested_message_order() {
        let command = Command::Batch(vec![
            Command::Batch(vec![
                Command::message("first"),
                Command::request_repaint(),
                Command::Batch(vec![
                    Command::message("second"),
                    Command::none(),
                    Command::Batch(vec![Command::message("third")]),
                ]),
            ]),
            Command::Batch(vec![
                Command::request_paint_only(),
                Command::message("second"),
            ]),
        ]);

        let messages = command.into_messages();

        assert_eq!(messages, vec!["first", "second", "third", "second"]);
        assert!(messages.capacity() >= messages.len());
    }

    #[test]
    fn batch_collapses_single_command_groups() {
        let command = Command::batch([Command::none(), Command::batch([Command::message("only")])]);

        assert!(matches!(command, Command::Message("only")));
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
