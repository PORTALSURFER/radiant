use crate::{
    gui::types::Point,
    runtime::{Command, RuntimeUpdateSnapshot},
};

mod business;
mod commands;
mod platform;
mod surface;

pub use business::{BusinessEventSink, BusinessRuntime, BusinessWorkContext};
pub(in crate::application) use business::{
    BusinessWorkDiagnosticSummary, with_business_work_diagnostics,
};

/// UI-safe context supplied to app message handlers.
///
/// This context is the only normal app-facing capability surface for
/// runtime-visible follow-up work. It exposes repaint/focus/timer/platform
/// requests and business-work scheduling, but not arbitrary command injection
/// or worker-only business capabilities.
pub struct UiUpdateContext<Message> {
    commands: Vec<Command<Message>>,
    runtime_snapshot: RuntimeUpdateSnapshot,
}

impl<Message> Default for UiUpdateContext<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            runtime_snapshot: RuntimeUpdateSnapshot::default(),
        }
    }
}

impl<Message> UiUpdateContext<Message> {
    /// Build an update context around a runtime-owned input snapshot.
    pub fn from_runtime_snapshot(runtime_snapshot: RuntimeUpdateSnapshot) -> Self {
        Self {
            commands: Vec::new(),
            runtime_snapshot,
        }
    }

    pub(in crate::application) fn queue_command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Latest logical pointer position known to the runtime for this update.
    pub fn current_pointer_position(&self) -> Option<Point> {
        self.runtime_snapshot.current_pointer_position()
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
