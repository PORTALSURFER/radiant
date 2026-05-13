//! Message-reducing declarative runtime bridges.

use super::super::RuntimeBridge;
use crate::runtime::UiSurface;
use std::sync::Arc;

/// Closure-driven bridge for generic declarative Radiant hosts.
///
/// The bridge owns one state value and delegates:
/// - view projection to `project`
/// - host-message reduction to `reduce`
///
/// This preserves one-way data flow:
/// `state --(project)--> surface`, `message --(reduce)--> state`.
pub struct DeclarativeRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    state: State,
    project: Project,
    reduce: Reduce,
}

impl<State, Message, Project, Reduce> DeclarativeRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build a generic declarative bridge from state, projector, and reducer closures.
    pub fn new(state: State, project: Project, reduce: Reduce) -> Self {
        Self {
            state,
            project,
            reduce,
        }
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
    for DeclarativeRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        (self.project)(&mut self.state)
    }

    fn reduce_message(&mut self, message: Message) {
        (self.reduce)(&mut self.state, message);
    }
}

/// Closure-driven bridge for generic hosts that project owned surface snapshots.
///
/// This is the allocation-lean counterpart to [`DeclarativeRuntimeBridge`] for
/// hosts whose projector naturally builds a fresh [`UiSurface`] for each
/// runtime refresh. Shared-surface hosts can continue using
/// [`declarative_runtime_bridge`].
pub struct DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    state: State,
    project: Project,
    reduce: Reduce,
}

impl<State, Message, Project, Reduce> DeclarativeOwnedRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> UiSurface<Message>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build an owned-surface bridge from state, projector, and reducer closures.
    pub fn new(state: State, project: Project, reduce: Reduce) -> Self {
        Self {
            state,
            project,
            reduce,
        }
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

/// Build a closure-driven declarative bridge for a generic message-driven surface.
pub fn declarative_runtime_bridge<State, Message, Project, Reduce>(
    state: State,
    project: Project,
    reduce: Reduce,
) -> DeclarativeRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    DeclarativeRuntimeBridge::new(state, project, reduce)
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
