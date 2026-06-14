use crate::runtime::Command;

mod business;
mod commands;
mod platform;
mod surface;

pub(in crate::application) use business::with_business_work_diagnostics;
pub use business::{BusinessEventSink, BusinessRuntime, BusinessWorkContext};

/// UI-safe context supplied to app message handlers.
///
/// This context is the only normal app-facing capability surface for
/// runtime-visible follow-up work. It exposes repaint/focus/timer/platform
/// requests and business-work scheduling, but not arbitrary command injection
/// or worker-only business capabilities.
pub struct UiUpdateContext<Message> {
    commands: Vec<Command<Message>>,
}

impl<Message> Default for UiUpdateContext<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<Message> UiUpdateContext<Message> {
    pub(in crate::application) fn queue_command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Access Radiant's business-work submission API.
    ///
    /// Use this for host-owned IO, decoding, cache hydration, persistence,
    /// analysis, and other work that must not run on the UI/event/render path.
    pub fn business(&mut self) -> BusinessRuntime<'_, Message> {
        BusinessRuntime::new(self)
    }

    /// Consume this UI update context into the batched runtime command it queued.
    ///
    /// Most apps use Radiant's app builders, which collect this automatically.
    /// This method is for custom runtime bridges and tests that call a reducer
    /// directly but still need to execute queued runtime work.
    pub fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
