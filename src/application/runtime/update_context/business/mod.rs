mod keyed_latest;
mod latest;
mod request;
mod resource;
mod sink;
mod work_context;

use crate::runtime::TaskPriority;

use request::BusinessRequest;

use super::UiUpdateContext;

pub use sink::BusinessEventSink;
pub use work_context::BusinessWorkContext;
pub(in crate::application) use work_context::{
    BusinessWorkDiagnosticSummary, with_business_work_diagnostics,
};

/// UI-update access point for submitting host-owned business work.
pub struct BusinessRuntime<'context, Message> {
    context: &'context mut UiUpdateContext<Message>,
}

impl<'context, Message> BusinessRuntime<'context, Message> {
    pub(super) fn new(context: &'context mut UiUpdateContext<Message>) -> Self {
        Self { context }
    }

    /// Submit user-visible work that should complete promptly off the UI path.
    pub fn interactive(self, name: &'static str) -> BusinessRequest<'context, Message> {
        self.request(name, TaskPriority::Interactive)
    }

    /// Submit work on a host-selected priority lane.
    ///
    /// Prefer the named lane helpers when the priority is static at the call
    /// site. Use this helper when host policy already resolved the lane and
    /// the update handler should still schedule the work through Radiant's
    /// explicit business-runtime boundary.
    pub fn priority(
        self,
        name: &'static str,
        priority: TaskPriority,
    ) -> BusinessRequest<'context, Message> {
        self.request(name, priority)
    }

    /// Submit ordinary background work off the UI path.
    pub fn background(self, name: &'static str) -> BusinessRequest<'context, Message> {
        self.request(name, TaskPriority::Background)
    }

    /// Submit explicit blocking IO work off the UI path on a limited lane.
    pub fn blocking_io(self, name: &'static str) -> BusinessRequest<'context, Message> {
        self.request(name, TaskPriority::BlockingIo)
    }

    /// Submit opportunistic work that may yield to interactive/background work.
    pub fn idle(self, name: &'static str) -> BusinessRequest<'context, Message> {
        self.request(name, TaskPriority::Idle)
    }

    fn request(
        self,
        name: &'static str,
        priority: TaskPriority,
    ) -> BusinessRequest<'context, Message> {
        BusinessRequest {
            context: self.context,
            name,
            priority,
        }
    }
}
