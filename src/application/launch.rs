//! Window and stateful application launch builders.

use super::{
    AppAnimation, AppBridge, AppBridgeLifecycle, AppCloseRequested, AppFrameMessage, AppShortcuts,
    AppShutdown, AppStartup, AppSubscriptions, AppUpdate, Result, RetainedPainter, StateAction,
    UpdateContext,
};
use crate::{
    gui_runtime::{NativeRunOptions, WindowSpec},
    runtime::{
        Command, RuntimeBridge, SurfaceNode, UiSurface, declarative_command_runtime_bridge,
        run_native_vello_runtime,
    },
};
use std::{collections::HashMap, marker::PhantomData, sync::Arc};

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

pub use into_view::IntoView;
pub use stateful::{RunnableStatefulApp, StatefulAppBuilder, StatefulAppWithView};
pub use window::WindowBuilder;
