//! Generic declarative bridge traits for message-driven Radiant hosts.

use super::{Command, surface::UiSurface};
use crate::gui::repaint::RepaintSignal;
use std::sync::Arc;

/// Generic host/runtime bridge for declarative message-driven surfaces.
///
/// The host projects one immutable [`UiSurface`] snapshot per frame and reduces
/// host-defined messages emitted by widgets back into owned application state.
pub trait RuntimeBridge<Message> {
    /// Project the latest immutable UI surface snapshot.
    fn project_surface(&mut self) -> Arc<UiSurface<Message>>;

    /// Pull the latest immutable UI surface snapshot as an owned value.
    fn pull_surface(&mut self) -> UiSurface<Message> {
        Arc::unwrap_or_clone(self.project_surface())
    }

    /// Reduce one host-defined message into application state.
    fn reduce_message(&mut self, _message: Message) {}

    /// Update application state and return runtime-visible follow-up work.
    ///
    /// Existing hosts can keep implementing [`RuntimeBridge::reduce_message`].
    /// Hosts that need command dispatch can override this method and return
    /// [`Command`] values without moving side-effect ownership into Radiant.
    fn update(&mut self, message: Message) -> Command<Message> {
        self.reduce_message(message);
        Command::none()
    }

    /// Install a repaint signal that host-owned background work can use to wake
    /// the native runtime after asynchronous state changes.
    ///
    /// Declarative hosts that do not run background work can rely on the
    /// default no-op implementation. Hosts that do should store this signal and
    /// forward it to their worker systems rather than depending on backend
    /// internals.
    fn install_repaint_signal(&mut self, _signal: Arc<dyn RepaintSignal>) {}
}

/// Public application contract for declarative Radiant hosts.
///
/// `App` is a named API concept for any host object that can project a
/// [`UiSurface`] and reduce widget-emitted messages. It is implemented
/// automatically for every [`RuntimeBridge`], so existing closure-driven and
/// custom bridge hosts remain allocation-free and do not need adapter wrappers.
pub trait App<Message>: RuntimeBridge<Message> {}

impl<Bridge, Message> App<Message> for Bridge where Bridge: RuntimeBridge<Message> {}

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
