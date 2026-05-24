use super::super::super::RuntimeBridge;
use crate::runtime::UiSurface;
use std::sync::Arc;

/// Closure-driven bridge for generic hosts that project owned surface snapshots.
///
/// This is the allocation-lean counterpart to [`super::DeclarativeRuntimeBridge`] for
/// hosts whose projector naturally builds a fresh [`UiSurface`] for each
/// runtime refresh. Shared-surface hosts can continue using
/// [`super::declarative_runtime_bridge`].
pub struct DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    state: State,
    project: Project,
    reduce: Reduce,
}

/// Named construction fields for a [`DeclarativeOwnedRuntimeBridge`].
pub struct DeclarativeOwnedRuntimeBridgeParts<State, Project, Reduce> {
    /// Host-owned state projected into a UI surface.
    pub state: State,
    /// Closure that projects state into an owned surface snapshot.
    pub project: Project,
    /// Closure that reduces host messages into state updates.
    pub reduce: Reduce,
}

impl<State, Message, Project, Reduce> DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build an owned-surface bridge from named parts.
    pub fn from_parts(parts: DeclarativeOwnedRuntimeBridgeParts<State, Project, Reduce>) -> Self {
        Self {
            state: parts.state,
            project: parts.project,
            reduce: parts.reduce,
        }
    }

    /// Build an owned-surface bridge from state, projector, and reducer closures.
    pub fn new(state: State, project: Project, reduce: Reduce) -> Self {
        Self::from_parts(DeclarativeOwnedRuntimeBridgeParts {
            state,
            project,
            reduce,
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

impl<State, Message, Project, Reduce> RuntimeBridge<Message>
    for DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::new((self.project)(&mut self.state))
    }

    fn pull_surface(&mut self) -> UiSurface<Message> {
        (self.project)(&mut self.state)
    }

    fn reduce_message(&mut self, message: Message) {
        (self.reduce)(&mut self.state, message);
    }
}

/// Build a closure-driven declarative bridge from owned surface snapshots.
pub fn declarative_owned_runtime_bridge<State, Message, Project, Reduce>(
    state: State,
    project: Project,
    reduce: Reduce,
) -> DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    DeclarativeOwnedRuntimeBridge::new(state, project, reduce)
}
