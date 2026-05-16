use super::Command;
use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::{
        ExternalDragOutcome, ExternalDragRequest, PlatformRequest, PlatformResponse, RepaintScope,
    },
    widgets::WidgetId,
};
use std::time::Duration;

impl<Message> Command<Message> {
    /// Return an empty command.
    pub const fn none() -> Self {
        Self::None
    }

    /// Build a command that dispatches one host-defined message.
    pub const fn message(message: Message) -> Self {
        Self::Message(message)
    }

    /// Build a command that dispatches multiple commands in order.
    pub fn batch(command_iter: impl IntoIterator<Item = Command<Message>>) -> Self {
        let command_iter = command_iter.into_iter();
        let mut commands = Vec::with_capacity(command_iter.size_hint().0);
        for command in command_iter {
            command.append_to_batch(&mut commands);
        }
        match commands.len() {
            0 => Self::None,
            1 => match commands.pop() {
                Some(command) => command,
                None => Self::None,
            },
            _ => Self::Batch(commands),
        }
    }

    /// Build a command that asks the active runtime adapter to repaint.
    pub const fn request_repaint() -> Self {
        Self::RequestRepaint
    }

    /// Build a command that repaints without refreshing the declarative surface.
    pub const fn request_paint_only() -> Self {
        Self::RequestPaintOnly
    }

    /// Build a repaint command from a typed repaint scope.
    pub const fn repaint(scope: RepaintScope) -> Self {
        match scope {
            RepaintScope::Surface => Self::RequestRepaint,
            RepaintScope::PaintOnly => Self::RequestPaintOnly,
        }
    }

    /// Build a command that dispatches one message after the provided delay.
    pub const fn after(delay: Duration, message: Message) -> Self {
        Self::After { delay, message }
    }

    /// Build a command that runs work on a runtime-managed business thread and
    /// maps its result into a host message.
    ///
    /// Use this for IO, decoding, analysis, slow computation, and other work
    /// that should not block the UI/event/render path. If synchronous execution
    /// is intentionally required, dispatch a normal [`Command::message`] and do
    /// that short work in the reducer instead.
    pub fn perform<Output>(
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::Perform {
            name,
            work: Box::new(move || map(work())),
        }
    }

    /// Build a command that moves keyboard focus to one widget.
    pub const fn focus(widget_id: WidgetId) -> Self {
        Self::Focus(widget_id)
    }

    /// Build a command that moves one scroll container to a logical offset.
    pub const fn scroll_to(node_id: NodeId, offset: Vector2) -> Self {
        Self::ScrollTo { node_id, offset }
    }

    /// Build a command that reveals a vertical span inside one scroll container.
    pub const fn scroll_into_view(
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
    ) -> Self {
        Self::ScrollIntoView {
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y: None,
        }
    }

    /// Build a command that reveals a vertical span and snaps movement to a fixed row height.
    pub const fn scroll_into_view_snapped(
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: f32,
    ) -> Self {
        Self::ScrollIntoView {
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y: Some(snap_y),
        }
    }

    /// Build a command that reveals a fixed-stride row with directional context rows.
    pub const fn scroll_fixed_row_into_view(
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) -> Self {
        Self::ScrollFixedRowIntoView {
            node_id,
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
        }
    }

    /// Build a command that arms a native external drag session.
    pub fn begin_external_drag(
        request: ExternalDragRequest,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + Send + 'static,
    ) -> Self {
        Self::BeginExternalDrag {
            request,
            on_completed: Some(Box::new(on_completed)),
        }
    }

    /// Build a command that arms a native external drag session without completion notification.
    pub fn begin_external_drag_without_completion(request: ExternalDragRequest) -> Self {
        Self::BeginExternalDrag {
            request,
            on_completed: None,
        }
    }

    /// Build a command that requests a platform service.
    pub fn platform_request(
        request: PlatformRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) -> Self {
        Self::PlatformRequest {
            request,
            on_completed: Box::new(on_completed),
        }
    }

    /// Build a command that clears any active native external drag session.
    pub const fn end_external_drag() -> Self {
        Self::EndExternalDrag
    }

    /// Build a command that asks the active runtime to exit.
    pub const fn exit() -> Self {
        Self::Exit
    }

    fn append_to_batch(self, commands: &mut Vec<Command<Message>>) {
        match self {
            Self::None => {}
            Self::Batch(nested) => {
                commands.reserve(nested.len());
                for command in nested {
                    command.append_to_batch(commands);
                }
            }
            command => commands.push(command),
        }
    }
}
