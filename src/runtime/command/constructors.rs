use super::Command;
use crate::widgets::WidgetId;
use std::time::Duration;

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
            1 => match commands.pop() {
                Some(command) => command,
                None => Self::None,
            },
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

    /// Build a command that runs work on a runtime-managed business thread and
    /// maps its result into a host message.
    ///
    /// Use this for IO, decoding, analysis, slow computation, and other work
    /// that should not block the UI/event/render path. If synchronous execution
    /// is intentionally required, dispatch a normal [`Command::message`] and do
    /// that short work in the reducer instead.
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
}
