use super::super::super::RuntimeBridge;
use crate::runtime::{Command, UiSurface};
use std::sync::Arc;

/// Closure-driven bridge for declarative hosts whose update returns commands.
///
/// This is the command-returning counterpart to
/// [`super::super::DeclarativeRuntimeBridge`]. It keeps host state and side
/// effects host-owned while allowing the generic Radiant runtime to observe
/// message chaining, command batches, and repaint requests.
pub struct DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    state: State,
    project: Project,
    update: Update,
}

/// Named construction fields for a [`DeclarativeCommandRuntimeBridge`].
pub struct DeclarativeCommandRuntimeBridgeParts<State, Project, Update> {
    /// Host-owned state projected into a UI surface.
    pub state: State,
    /// Closure that projects state into a shared surface snapshot.
    pub project: Project,
    /// Closure that reduces host messages and returns follow-up commands.
    pub update: Update,
}

impl<State, Message, Project, Update>
    DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    /// Build a command-returning declarative bridge from named parts.
    pub fn from_parts(parts: DeclarativeCommandRuntimeBridgeParts<State, Project, Update>) -> Self {
        Self {
            state: parts.state,
            project: parts.project,
            update: parts.update,
        }
    }

    /// Build a generic declarative bridge from state, projector, and command update closures.
    pub fn new(state: State, project: Project, update: Update) -> Self {
        Self::from_parts(DeclarativeCommandRuntimeBridgeParts {
            state,
            project,
            update,
        })
    }

    /// Return an immutable reference to the owned host state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Return a mutable reference to the owned host state.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Reduce one host message while discarding follow-up commands.
    pub fn reduce_message(&mut self, message: Message) {
        let _ = (self.update)(&mut self.state, message);
    }

    /// Consume the bridge and return the owned host state.
    pub fn into_state(self) -> State {
        self.state
    }
}

impl<State, Message, Project, Update> RuntimeBridge<Message>
    for DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        (self.project)(&mut self.state)
    }

    fn reduce_message(&mut self, message: Message) {
        DeclarativeCommandRuntimeBridge::reduce_message(self, message);
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        (self.update)(&mut self.state, message)
    }
}

/// Build a closure-driven declarative bridge whose update returns commands.
pub fn declarative_command_runtime_bridge<State, Message, Project, Update>(
    state: State,
    project: Project,
    update: Update,
) -> DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    DeclarativeCommandRuntimeBridge::new(state, project, update)
}
