//! Declarative bridge helpers for runtime hosts.
//!
//! This module provides a closure-driven adapter so hosts can wire
//! `state + reducer + projector` without writing imperative bridge types.

use super::{AppModel, NativeAppBridge, UiAction};
use std::sync::Arc;

/// Closure-driven native bridge for declarative host state reducers.
///
/// The bridge owns one state value and delegates:
/// - view projection to `project`
/// - action reduction to `reduce`
///
/// This keeps host integration in a declarative shape:
/// `state --(project)--> model`, `action --(reduce)--> state`.
pub struct DeclarativeBridge<State, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<AppModel>,
    Reduce: FnMut(&mut State, UiAction),
{
    state: State,
    project: Project,
    reduce: Reduce,
}

impl<State, Project, Reduce> DeclarativeBridge<State, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<AppModel>,
    Reduce: FnMut(&mut State, UiAction),
{
    /// Build a declarative bridge from state, projector, and reducer closures.
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

impl<State, Project, Reduce> NativeAppBridge for DeclarativeBridge<State, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<AppModel>,
    Reduce: FnMut(&mut State, UiAction),
{
    fn project_model(&mut self) -> Arc<AppModel> {
        (self.project)(&mut self.state)
    }

    fn reduce_action(&mut self, action: UiAction) {
        (self.reduce)(&mut self.state, action);
    }
}

/// Build a closure-driven declarative bridge.
pub fn declarative_bridge<State, Project, Reduce>(
    state: State,
    project: Project,
    reduce: Reduce,
) -> DeclarativeBridge<State, Project, Reduce>
where
    Project: FnMut(&mut State) -> Arc<AppModel>,
    Reduce: FnMut(&mut State, UiAction),
{
    DeclarativeBridge::new(state, project, reduce)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct CounterState {
        count: usize,
    }

    #[test]
    fn declarative_bridge_projects_and_reduces_state() {
        let mut bridge = declarative_bridge(
            CounterState::default(),
            |state: &mut CounterState| {
                let mut model = AppModel::default();
                model.status_text = format!("count={}", state.count);
                Arc::new(model)
            },
            |state: &mut CounterState, _action: UiAction| {
                state.count = state.count.saturating_add(1);
            },
        );

        let model_before = bridge.project_model();
        assert_eq!(model_before.status_text, "count=0");
        bridge.reduce_action(UiAction::ToggleTransport);
        let model_after = bridge.project_model();
        assert_eq!(model_after.status_text, "count=1");
    }
}
