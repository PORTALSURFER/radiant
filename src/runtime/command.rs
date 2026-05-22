//! Generic command values returned or queued by host-side runtime code.

use super::drag::DragRequest;
use super::external_drag::{ExternalDragCompletion, ExternalDragRequest};
use super::platform::{PlatformCompletion, PlatformRequest};
use crate::{gui::types::Vector2, layout::NodeId, widgets::WidgetId};
use std::time::Duration;

mod constructors;
mod debug;
mod flatten;
mod query;
mod repaint;
mod scroll;

pub use repaint::RepaintScope;
pub use scroll::{ScrollFixedRowIntoViewParts, ScrollIntoViewParts};

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
    /// Move one scroll container to a logical offset.
    ScrollTo {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Requested logical scroll offset.
        offset: Vector2,
    },
    /// Reveal one vertical content span inside a scroll container.
    ScrollIntoView {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Logical top edge of the target span inside the scroll content.
        target_y: f32,
        /// Logical height of the target span.
        target_height: f32,
        /// Preferred space to keep above the target.
        margin_top: f32,
        /// Preferred space to keep below the target.
        margin_bottom: f32,
        /// Optional vertical snap interval for fixed-row lists.
        snap_y: Option<f32>,
    },
    /// Reveal one fixed-stride row with directional context rows.
    ScrollFixedRowIntoView {
        /// Scroll container node to move.
        node_id: NodeId,
        /// Zero-based row index inside the scroll content.
        row_index: usize,
        /// Fixed distance between adjacent row starts in logical pixels.
        row_stride: f32,
        /// Rows to keep above the target while navigating upward.
        leading_context_rows: usize,
        /// Rows to keep below the target while navigating downward.
        trailing_context_rows: usize,
        /// Negative for upward navigation, positive for downward navigation.
        direction: i32,
    },
    /// Arm a native external drag session.
    ///
    /// Native backends launch the session when the active pointer drag leaves
    /// the application window, allowing external targets such as file managers
    /// to accept the payload.
    BeginExternalDrag {
        /// Payload and preview metadata for the native drag session.
        request: ExternalDragRequest,
        /// Optional host callback mapped into a message when the native drag loop ends.
        on_completed: Option<ExternalDragCompletion<Message>>,
    },
    /// Begin a runtime-owned pointer drag preview session.
    BeginDrag {
        /// Preview and initial pointer metadata.
        request: DragRequest,
    },
    /// End any active runtime-owned pointer drag preview session.
    EndDrag,
    /// Request a platform service such as a file picker or confirmation dialog.
    PlatformRequest {
        /// Platform service request.
        request: PlatformRequest,
        /// Host callback mapped into a message when the request completes.
        on_completed: PlatformCompletion<Message>,
    },
    /// Clear any active native external drag session.
    EndExternalDrag,
    /// Request that the active runtime exits.
    Exit,
}

#[cfg(test)]
mod tests;
