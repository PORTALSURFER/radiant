use crate::{
    application::{DeclarativeEffectOwner, LatestTask},
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
    ///
    /// The message is kept by the UI runtime until an opaque timer wake is
    /// drained on the UI turn; it is not constructed or transported by the
    /// timer thread. Use this for one delayed action that does not need
    /// replacement/latest semantics.
    pub fn after(&mut self, delay: std::time::Duration, message: Message)
    where
        Message: 'static,
    {
        self.queue_command(Command::after(delay, message));
    }

    /// Dispatch a delayed message only while the explicitly named declarative
    /// owner remains one current eligible source. Rejection is fail-closed;
    /// this method never falls back to application ownership.
    pub fn after_for_owner(
        &mut self,
        owner: DeclarativeEffectOwner,
        delay: std::time::Duration,
        message: Message,
    ) where
        Message: 'static,
    {
        self.queue_command(Command::after_for_owner(owner, delay, message));
    }

    /// Dispatch a delayed message tagged with a latest-task ticket.
    ///
    /// Calling this method replaces the pending delay for `latest`. The UI
    /// runtime invokes the mapper only for the still-active ticket after it
    /// drains and validates the opaque wake. The mapper and resulting message
    /// remain UI-owned; no message crosses the timer thread.
    pub fn after_latest(
        &mut self,
        latest: &mut LatestTask,
        delay: std::time::Duration,
        map: impl FnOnce(crate::application::TaskTicket) -> Message + 'static,
    ) where
        Message: 'static,
    {
        let transaction = latest.begin_timer_replacement();
        let ticket = transaction.replacement();
        self.queue_command(Command::after_latest(delay, ticket, transaction, map));
    }

    /// Dispatch a replaceable delayed message only while the explicitly named
    /// declarative owner remains one current eligible source. Rejected owner
    /// admission rolls back the latest-task transaction and invokes no mapper.
    pub fn after_latest_for_owner(
        &mut self,
        owner: DeclarativeEffectOwner,
        latest: &mut LatestTask,
        delay: std::time::Duration,
        map: impl FnOnce(crate::application::TaskTicket) -> Message + 'static,
    ) where
        Message: 'static,
    {
        let transaction = latest.begin_timer_replacement();
        let ticket = transaction.replacement();
        self.queue_command(Command::after_latest_for_owner(
            owner,
            delay,
            ticket,
            transaction,
            map,
        ));
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
