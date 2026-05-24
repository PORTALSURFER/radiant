use super::super::super::RuntimeBridge;
use crate::runtime::{Command, UiSurface};
use std::sync::Arc;

/// Closure-driven command bridge for hosts that project owned surface snapshots.
pub struct DeclarativeOwnedCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    state: State,
    project: Project,
    update: Update,
}

/// Named construction fields for a [`DeclarativeOwnedCommandRuntimeBridge`].
pub struct DeclarativeOwnedCommandRuntimeBridgeParts<State, Project, Update> {
    /// Host-owned state projected into a UI surface.
    pub state: State,
    /// Closure that projects state into an owned surface snapshot.
    pub project: Project,
    /// Closure that reduces host messages and returns follow-up commands.
    pub update: Update,
}

impl<State, Message, Project, Update>
    DeclarativeOwnedCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    /// Build an owned-surface command bridge from named parts.
    pub fn from_parts(
        parts: DeclarativeOwnedCommandRuntimeBridgeParts<State, Project, Update>,
    ) -> Self {
        Self {
            state: parts.state,
            project: parts.project,
            update: parts.update,
        }
    }

    /// Build an owned-surface command bridge from state, projector, and update closures.
    pub fn new(state: State, project: Project, update: Update) -> Self {
        Self::from_parts(DeclarativeOwnedCommandRuntimeBridgeParts {
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

    /// Consume the bridge and return the owned host state.
    pub fn into_state(self) -> State {
        self.state
    }
}

impl<State, Message, Project, Update> RuntimeBridge<Message>
    for DeclarativeOwnedCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new((self.project)(&mut self.state))
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        (self.project)(&mut self.state)
    }

    fn reduce_message(&mut self, message: Message) {
        let _ = (self.update)(&mut self.state, message);
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        (self.update)(&mut self.state, message)
    }
}

/// Build a command-returning declarative bridge from owned surface snapshots.
pub fn declarative_owned_command_runtime_bridge<State, Message, Project, Update>(
    state: State,
    project: Project,
    update: Update,
) -> DeclarativeOwnedCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    DeclarativeOwnedCommandRuntimeBridge::new(state, project, update)
}
