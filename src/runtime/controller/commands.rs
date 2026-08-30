use super::SurfaceRuntime;
use super::declarative_owner::DeclarativeOwnerRequest;
use super::owner::{AuxiliaryWindowOwner, EffectOrigin};
use crate::runtime::{Command, RuntimeBridge};
use crate::widgets::WidgetId;

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

/// One focus command staged by an auxiliary-origin message.
///
/// The owner is retained alongside the command so the native synchronization
/// boundary can reject a retired generation without looking up a replacement
/// by key alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuxiliaryFocusCommand {
    Focus(WidgetId),
    ClearFocus,
}

pub(crate) struct AuxiliaryFocusIntent {
    owner: AuxiliaryWindowOwner,
    command: AuxiliaryFocusCommand,
}

impl AuxiliaryFocusIntent {
    pub(crate) fn new(owner: AuxiliaryWindowOwner, command: AuxiliaryFocusCommand) -> Self {
        Self { owner, command }
    }

    pub(crate) fn owner(&self) -> &AuxiliaryWindowOwner {
        &self.owner
    }

    pub(crate) const fn command(&self) -> AuxiliaryFocusCommand {
        self.command
    }
}

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

    pub(crate) fn take_auxiliary_focus_intents(&mut self) -> Vec<AuxiliaryFocusIntent> {
        std::mem::take(&mut self.auxiliary_focus_intents)
    }

    pub(crate) fn auxiliary_focus_intents_pending(&self) -> bool {
        !self.auxiliary_focus_intents.is_empty()
    }

    fn stage_auxiliary_focus_intents(
        &mut self,
        owner: AuxiliaryWindowOwner,
        commands: impl IntoIterator<Item = AuxiliaryFocusCommand>,
    ) {
        self.auxiliary_focus_intents.extend(
            commands
                .into_iter()
                .map(|command| AuxiliaryFocusIntent::new(owner.clone(), command)),
        );
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
