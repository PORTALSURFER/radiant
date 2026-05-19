use crate::runtime::{Command, RepaintScope};

use super::UpdateContext;

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

    /// Request a repaint using an explicit repaint scope.
    pub fn repaint(&mut self, scope: RepaintScope) {
        self.command(Command::repaint(scope));
    }

    /// Dispatch a message after a delay.
    pub fn after(&mut self, delay: std::time::Duration, message: Message) {
        self.command(Command::after(delay, message));
    }

    /// Request runtime exit.
    pub fn exit(&mut self) {
        self.command(Command::exit());
    }
}
