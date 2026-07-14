use super::super::super::RuntimeBridge;
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

/// Named construction fields for a [`DeclarativeRuntimeBridge`].
pub struct DeclarativeRuntimeBridgeParts<State, Project, Reduce> {
    /// Host-owned state projected into a UI surface.
    pub state: State,
    /// Closure that projects state into a shared surface snapshot.
    pub project: Project,
    /// Closure that reduces host messages into state updates.
    pub reduce: Reduce,
}

impl<State, Message, Project, Reduce> DeclarativeRuntimeBridge<State, Message, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<UiSurface<Message>>,
    Reduce: FnMut(&mut State, Message),
{
    /// Build a generic declarative bridge from named parts.
    pub fn from_parts(parts: DeclarativeRuntimeBridgeParts<State, Project, Reduce>) -> Self {
        Self {
            state: parts.state,
            project: parts.project,
            reduce: parts.reduce,
        }
    }

    /// Build a generic declarative bridge from state, projector, and reducer closures.
    pub fn new(state: State, project: Project, reduce: Reduce) -> Self {
        Self::from_parts(DeclarativeRuntimeBridgeParts {
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

    /// Reduce one host message into the owned state.
    pub fn reduce_message(&mut self, message: Message) {
        (self.reduce)(&mut self.state, message);
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
        DeclarativeRuntimeBridge::reduce_message(self, message);
    }

    fn update(&mut self, message: Message) -> crate::runtime::Command<Message> {
        self.reduce_message(message);
        crate::runtime::Command::none()
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
