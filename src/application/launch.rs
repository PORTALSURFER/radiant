//! Window and stateful application launch builders.

/// Build a native window launcher for a simple Radiant view.
pub fn window(title: impl Into<String>) -> WindowBuilder {
    WindowBuilder::new(title)
}

/// Build a stateful app launcher over the existing command runtime bridge.
pub fn app<State>(state: State) -> StatefulAppBuilder<State> {
    StatefulAppBuilder::new(state)
}

mod into_view;
mod stateful;
mod window;

pub(in crate::application) use into_view::SceneProjection;
pub use into_view::{IntoView, ViewProjection};
pub use stateful::{RunnableStatefulApp, StatefulAppBuilder, StatefulAppWithView};
pub use window::WindowBuilder;
