//! Stateful-application runtime prelude exports.

pub use crate::Result;
pub use crate::application::{
    CancellationToken, KeyedLatestTasks, KeyedTaskCompletion, LatestTask, RunnableStatefulApp,
    StateAction, StateView, StatefulAppBuilder, StatefulAppWithView, Subscription, TaskCompletion,
    TaskTicket, UpdateContext, WindowBuilder, app, window,
};
