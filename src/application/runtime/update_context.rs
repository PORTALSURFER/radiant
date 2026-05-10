use crate::{runtime::Command, widgets::WidgetId};
use std::time::Duration;

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
    /// Queue a command produced by the current update.
    pub fn command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Queue a host-defined message.
    pub fn emit(&mut self, message: Message) {
        self.command(Command::message(message));
    }

    /// Request another repaint from the active runtime.
    pub fn request_repaint(&mut self) {
        self.command(Command::request_repaint());
    }

    /// Request repaint without forcing declarative surface reprojection.
    pub fn request_paint_only(&mut self) {
        self.command(Command::request_paint_only());
    }

    /// Dispatch a message after a delay.
    pub fn after(&mut self, delay: Duration, message: Message) {
        self.command(Command::after(delay, message));
    }

    /// Run background work and map the output into a host message.
    pub fn spawn<Output>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.command(Command::perform(name, work, map));
    }

    /// Move keyboard focus to a widget.
    pub fn focus(&mut self, widget_id: WidgetId) {
        self.command(Command::focus(widget_id));
    }

    /// Request runtime exit.
    pub fn exit(&mut self) {
        self.command(Command::exit());
    }

    pub(super) fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
