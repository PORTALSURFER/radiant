/// Build a native window launcher for a simple Radiant view.
pub fn window(title: impl Into<String>) -> WindowBuilder {
    WindowBuilder::new(title)
}

/// Build a stateful app launcher over the existing command runtime bridge.
pub fn app<State>(state: State) -> StatefulAppBuilder<State> {
    StatefulAppBuilder::new(state)
}

include!("launch/into_view.rs");
include!("launch/window.rs");
include!("launch/stateful.rs");
