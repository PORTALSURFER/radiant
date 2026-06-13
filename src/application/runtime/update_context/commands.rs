use crate::{
    application::LatestTask,
    layout::Vector2,
    runtime::{Command, RepaintScope},
    theme::DpiScale,
};

use super::UiUpdateContext;

impl<Message> UiUpdateContext<Message> {
    /// Queue a host-defined message.
    pub fn emit(&mut self, message: Message) {
        self.queue_command(Command::message(message));
    }

    /// Request another repaint from the active runtime.
    pub fn request_repaint(&mut self) {
        self.queue_command(Command::request_repaint());
    }

    /// Request repaint without forcing declarative surface reprojection.
    pub fn request_paint_only(&mut self) {
        self.queue_command(Command::request_paint_only());
    }

    /// Request a repaint using an explicit repaint scope.
    pub fn repaint(&mut self, scope: RepaintScope) {
        self.queue_command(Command::repaint(scope));
    }

    /// Dispatch a message after a delay.
    pub fn after(&mut self, delay: std::time::Duration, message: Message) {
        self.queue_command(Command::after(delay, message));
    }

    /// Dispatch a delayed message tagged with a latest-task ticket.
    pub fn after_latest(
        &mut self,
        latest: &mut LatestTask,
        delay: std::time::Duration,
        map: impl FnOnce(crate::application::TaskTicket) -> Message,
    ) {
        let ticket = latest.begin();
        self.after(delay, map(ticket));
    }

    /// Request runtime exit.
    pub fn exit(&mut self) {
        self.queue_command(Command::exit());
    }

    /// Set the runtime DPI scale override.
    pub fn set_dpi_scale(&mut self, scale: DpiScale) {
        self.queue_command(Command::set_dpi_scale(scale));
    }

    /// Set the runtime window size in logical points.
    pub fn set_window_logical_size(&mut self, size: Vector2) {
        self.queue_command(Command::set_window_logical_size(size));
    }
}
