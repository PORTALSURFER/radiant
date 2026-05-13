//! Generic command values returned or queued by host-side runtime code.

use crate::widgets::WidgetId;
use std::time::Duration;

mod constructors;
mod debug;
mod flatten;
mod query;

/// Runtime-facing command produced by host application logic.
///
/// Radiant commands are intentionally small and domain-neutral. Hosts keep
/// ownership of IO, background work, and other side effects; this type only
/// represents values the generic runtime can understand directly.
///
/// UI reducers should stay short and non-blocking. Expensive work belongs in
/// [`Command::perform`], which the application runtime offloads to a
/// runtime-managed business thread before delivering the resulting message back
/// through the normal UI update path.
#[derive(Default)]
pub enum Command<Message> {
    /// No follow-up work is required.
    #[default]
    None,
    /// Dispatch a host-defined message.
    Message(Message),
    /// Dispatch multiple commands in order.
    Batch(Vec<Command<Message>>),
    /// Request another redraw from the active runtime adapter.
    RequestRepaint,
    /// Request redraw without forcing declarative surface reprojection.
    RequestPaintOnly,
    /// Dispatch a host-defined message after a delay.
    After {
        /// Delay before the message is delivered.
        delay: Duration,
        /// Message to dispatch.
        message: Message,
    },
    /// Run host work on a business thread and dispatch the resulting message.
    Perform {
        /// Human-readable task name for diagnostics.
        name: &'static str,
        /// Background work lowered into a message-producing closure.
        work: Box<dyn FnOnce() -> Message + Send + 'static>,
    },
    /// Move keyboard focus to one widget.
    Focus(WidgetId),
    /// Request that the active runtime exits.
    Exit,
}

#[cfg(test)]
mod tests;
