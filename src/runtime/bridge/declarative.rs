//! Closure-driven runtime bridge implementations.

use super::RuntimeBridge;
use crate::runtime::{Command, UiSurface};
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

/// Closure-driven bridge for declarative hosts whose update returns commands.
///
/// This is the command-returning counterpart to [`DeclarativeRuntimeBridge`].
/// It keeps host state and side effects host-owned while allowing the generic
/// Radiant runtime to observe message chaining, command batches, and repaint
/// requests.
pub struct DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    state: State,
    project: Project,
    update: Update,
}

impl<State, Message, Project, Update>
    DeclarativeCommandRuntimeBridge<State, Message, Project, Update>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Update: FnMut(&mut State, Message) -> Command<Message>,
{
    /// Build a generic declarative bridge from state, projector, and command update closures.
    pub fn new(state: State, project: Project, update: Update) -> Self {
        Self {
            state,
            project,
            update,
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
        let _ = (self.update)(&mut self.state, message);
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
