use super::{Command, TaskPriority};
use crate::{
    runtime::{
        DragRequest, ExternalDragOutcome, ExternalDragRequest, PlatformRequest, PlatformResult,
        RepaintScope,
    },
    theme::DpiScale,
    widgets::WidgetId,
};
use std::time::Duration;

mod scroll;

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

    /// Build a command that overrides native DPI scale for the active runtime adapter.
    pub const fn set_dpi_scale(scale: DpiScale) -> Self {
        Self::SetDpiScale(scale)
    }

    /// Build a command that requests a native-window logical viewport size.
    pub const fn set_window_logical_size(size: crate::layout::Vector2) -> Self {
        Self::SetWindowLogicalSize(size)
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
        Self::perform_with_priority(name, TaskPriority::Background, work, map)
    }

    /// Build a background-work command with an explicit runtime scheduling
    /// priority hint.
    pub fn perform_with_priority<Output>(
        name: &'static str,
        priority: TaskPriority,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) -> Self
    where
        Output: Send + 'static,
    {
        Self::Perform {
            name,
            priority,
            work: Box::new(move || map(work())),
        }
    }

    /// Build a command that moves keyboard focus to one widget.
    pub const fn focus(widget_id: WidgetId) -> Self {
        Self::Focus(widget_id)
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

    /// Build a command that begins an in-window drag preview and arms a native
    /// external drag payload as one drag session.
    pub fn begin_drag_with_external(
        drag: DragRequest,
        external: ExternalDragRequest,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + Send + 'static,
    ) -> Self {
        Self::batch([
            Self::begin_drag(drag),
            Self::begin_external_drag(external, on_completed),
        ])
    }

    /// Build the commands needed to begin any available drag-session surfaces.
    ///
    /// This is useful when a host gesture may have an in-window preview, a
    /// native external-drag payload, both, or neither. The returned command is
    /// empty when both requests are `None`.
    pub fn begin_drag_session(
        drag: Option<DragRequest>,
        external: Option<ExternalDragRequest>,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + Send + 'static,
    ) -> Self {
        match (drag, external) {
            (Some(drag), Some(external)) => {
                Self::begin_drag_with_external(drag, external, on_completed)
            }
            (Some(drag), None) => Self::begin_drag(drag),
            (None, Some(external)) => Self::begin_external_drag(external, on_completed),
            (None, None) => Self::none(),
        }
    }

    /// Build a command that arms a native external drag session without completion notification.
    pub fn begin_external_drag_without_completion(request: ExternalDragRequest) -> Self {
        Self::BeginExternalDrag {
            request,
            on_completed: None,
        }
    }

    /// Build a command that begins a runtime-owned pointer drag preview.
    pub const fn begin_drag(request: DragRequest) -> Self {
        Self::BeginDrag { request }
    }

    /// Build a command that clears any active runtime-owned pointer drag preview.
    pub const fn end_drag() -> Self {
        Self::EndDrag
    }

    /// Build a command that requests a platform service.
    pub fn platform_request(
        request: PlatformRequest,
        on_completed: impl FnOnce(PlatformResult) -> Message + Send + 'static,
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
