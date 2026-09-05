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
    /// Resolve a semantic activation against current host state and reduce at most one initial message.
    ///
    /// Keyboard adapters call this after required text/IME handling and before legacy
    /// shortcut fallback. Only `Unhandled` permits fallback. Presentation targets are
    /// revalidated by the registered router before the mapper or reducer runs.
    pub fn dispatch_command_request(
        &mut self,
        request: crate::application::CommandRequest<'_>,
        surface: crate::gui::focus::FocusSurface,
    ) -> (crate::application::CommandDispatchStatus, CommandOutcome) {
        if !self.lifecycle_accepts_work() {
            return (
                crate::application::CommandDispatchStatus::Unavailable,
                CommandOutcome::default(),
            );
        }
        let focus = crate::application::CommandFocus {
            widget: self.focused_widget(),
            surface,
        };
        let dispatch = self.host_capabilities.input.as_ref().map_or_else(
            crate::application::CommandDispatch::unhandled,
            |capability| (capability.resolve_command)(&mut self.bridge, request, focus),
        );
        let outcome = dispatch
            .message
            .map_or_else(CommandOutcome::default, |message| {
                self.dispatch_message(message)
            });
        (dispatch.status, outcome)
    }

    pub(crate) fn dispatch_semantic_key_input(
        &mut self,
        input: &crate::application::CommandInput,
        surface: crate::gui::focus::FocusSurface,
    ) -> bool {
        let (status, outcome) = self
            .dispatch_command_request(crate::application::CommandRequest::Input(input), surface);
        self.pending_input_command_outcome.merge(outcome);
        let handled = status != crate::application::CommandDispatchStatus::Unhandled;
        if handled {
            self.interaction.focus.pending_key_chord = None;
        }
        handled
    }

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
