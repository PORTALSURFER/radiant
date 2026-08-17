use super::SurfaceRuntime;
use super::declarative_owner::DeclarativeOwnerRequest;
use super::owner::EffectOrigin;
use crate::runtime::{Command, RuntimeBridge};

#[cfg(test)]
pub(super) use crate::{
    gui::types::{Point, Vector2},
    runtime::UiSurface,
};

pub(super) mod batching;
mod dispatch;
mod drain;
mod external_drag;
mod outcome;
mod scrolling;

pub use outcome::CommandOutcome;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reduce one host-defined message and execute its runtime-visible command.
    pub fn dispatch_message(&mut self, message: Message) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner(message, &mut outcome);
        self.finish_command_outcome(outcome)
    }

    pub(crate) fn dispatch_message_from_auxiliary(
        &mut self,
        message: Message,
        owner: crate::runtime::AuxiliaryWindowOwner,
    ) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner_with_origin(
            message,
            &mut outcome,
            EffectOrigin::Auxiliary(owner),
        );
        self.finish_command_outcome(outcome)
    }

    /// Reduce one message under an explicit private declarative owner request.
    ///
    /// The request is resolved against the accepted source projection and its
    /// current live-generation ledger before the update handler runs. A
    /// rejected scoped request returns an empty outcome and cannot register
    /// follow-up work.
    #[allow(dead_code)]
    pub(crate) fn dispatch_message_from_declarative_owner(
        &mut self,
        message: Message,
        request: DeclarativeOwnerRequest,
        source_node: crate::layout::NodeId,
    ) -> CommandOutcome {
        let Some(origin) = self.declarative_owner_origin(request, source_node) else {
            return CommandOutcome::default();
        };
        let mut outcome = CommandOutcome::default();
        self.dispatch_message_inner_with_origin(message, &mut outcome, origin);
        self.finish_command_outcome(outcome)
    }

    /// Execute a command without an initial widget message.
    ///
    /// This is useful for backend adapters or host shells that need to replay a
    /// queued command through the same message/repaint handling path used by
    /// widget dispatch.
    pub fn execute_command(&mut self, command: Command<Message>) -> CommandOutcome {
        let mut outcome = CommandOutcome::default();
        if command.requires_fresh_surface_before_dispatch() && self.lifecycle_accepts_work() {
            outcome.surface_refresh_requested = true;
            self.execute_command_inner_deferred_refresh(command, &mut outcome);
        } else {
            self.execute_command_inner(command, &mut outcome);
        }
        self.finish_command_outcome(outcome)
    }
}

#[cfg(test)]
mod tests;
