use crate::{
    application::{KeyedLatestTasks, KeyedTaskCompletion, LatestTask, TaskCompletion},
    gui::types::Vector2,
    layout::NodeId,
    runtime::{
        Command, ConfirmDialogRequest, ExternalDragOutcome, ExternalDragRequest, FileDialogRequest,
        PlatformRequest, PlatformResponse, RepaintScope, ResourceCompletion, ResourceSlot,
    },
    widgets::WidgetId,
};
use std::time::Duration;

/// Context supplied to app update closures for runtime-visible follow-up work.
pub struct UpdateContext<Message> {
    commands: Vec<Command<Message>>,
}

impl<Message> Default for UpdateContext<Message> {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl<Message> UpdateContext<Message> {
    /// Queue a command produced by the current update.
    pub fn command(&mut self, command: Command<Message>) {
        self.commands.push(command);
    }

    /// Queue a host-defined message.
    pub fn emit(&mut self, message: Message) {
        self.command(Command::message(message));
    }

    /// Request another repaint from the active runtime.
    pub fn request_repaint(&mut self) {
        self.command(Command::request_repaint());
    }

    /// Request repaint without forcing declarative surface reprojection.
    pub fn request_paint_only(&mut self) {
        self.command(Command::request_paint_only());
    }

    /// Request a repaint using an explicit repaint scope.
    pub fn repaint(&mut self, scope: RepaintScope) {
        self.command(Command::repaint(scope));
    }

    /// Arm a native external drag session.
    ///
    /// Native backends launch the session when the active pointer drag leaves
    /// the application window.
    pub fn begin_external_drag(
        &mut self,
        request: ExternalDragRequest,
        on_completed: impl FnOnce(Result<ExternalDragOutcome, String>) -> Message + Send + 'static,
    ) {
        self.command(Command::begin_external_drag(request, on_completed));
    }

    /// Arm a native external drag session without completion notification.
    pub fn begin_external_drag_without_completion(&mut self, request: ExternalDragRequest) {
        self.command(Command::begin_external_drag_without_completion(request));
    }

    /// Clear any active native external drag session.
    pub fn end_external_drag(&mut self) {
        self.command(Command::end_external_drag());
    }

    /// Request a platform service through the active runtime bridge.
    pub fn platform_request(
        &mut self,
        request: PlatformRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.command(Command::platform_request(request, on_completed));
    }

    /// Ask the platform integration to choose a folder.
    pub fn pick_folder(
        &mut self,
        request: FileDialogRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::PickFolder(request), on_completed);
    }

    /// Ask the platform integration to show a confirmation dialog.
    pub fn confirm(
        &mut self,
        request: ConfirmDialogRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::Confirm(request), on_completed);
    }

    /// Dispatch a message after a delay.
    pub fn after(&mut self, delay: Duration, message: Message) {
        self.command(Command::after(delay, message));
    }

    /// Run work on a runtime-managed business thread and map the output into a
    /// host message.
    ///
    /// Use this for slow work so the UI thread remains responsive. The result
    /// returns through the normal message path after the work completes.
    pub fn spawn<Output>(
        &mut self,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(Output) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        self.command(Command::perform(name, work, map));
    }

    /// Start the latest task for one host-owned resource and run work on a
    /// runtime-managed business thread.
    ///
    /// The returned message receives a [`TaskCompletion`] tagged with the ticket
    /// created before the work started. Hosts can use [`LatestTask::finish`] to
    /// accept only the current completion and reject stale results.
    pub fn spawn_latest<Output>(
        &mut self,
        latest: &mut LatestTask,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let ticket = latest.begin();
        self.spawn(
            name,
            move || TaskCompletion {
                ticket,
                output: work(),
            },
            map,
        );
    }

    /// Start the latest task for one key in a keyed task registry and run work
    /// on a runtime-managed business thread.
    ///
    /// The returned message receives a keyed completion tagged with the key and
    /// ticket created before the work started. Hosts can use
    /// [`crate::application::KeyedLatestTasks::finish`] to accept only the
    /// current completion for that key and reject stale results.
    pub fn spawn_latest_for<Key, Output>(
        &mut self,
        latest: &mut KeyedLatestTasks<Key>,
        key: Key,
        name: &'static str,
        work: impl FnOnce() -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) where
        Key: Clone + Eq + std::hash::Hash + Send + 'static,
        Output: Send + 'static,
    {
        let ticket = latest.begin(key.clone());
        self.spawn(
            name,
            move || KeyedTaskCompletion {
                key,
                ticket,
                output: work(),
            },
            map,
        );
    }

    /// Start a resource load and run fallible work on a runtime-managed
    /// business thread.
    ///
    /// The returned message receives a [`ResourceCompletion`] tagged with the
    /// request created before the work started. Hosts should apply it with
    /// [`ResourceSlot::apply_for`] so older completions cannot overwrite newer
    /// requests for the same resource key.
    pub fn spawn_resource<Output>(
        &mut self,
        slot: &mut ResourceSlot<Output>,
        name: &'static str,
        work: impl FnOnce() -> Result<Output, String> + Send + 'static,
        map: impl FnOnce(ResourceCompletion<Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let request = slot.begin_load();
        self.spawn(
            name,
            move || {
                let load = match work() {
                    Ok(value) => request.ready(value),
                    Err(error) => request.failed(error),
                };
                ResourceCompletion::new(request, load)
            },
            map,
        );
    }

    /// Move keyboard focus to a widget.
    pub fn focus(&mut self, widget_id: WidgetId) {
        self.command(Command::focus(widget_id));
    }

    /// Move one scroll container to a logical offset.
    pub fn scroll_to(&mut self, node_id: NodeId, offset: Vector2) {
        self.command(Command::scroll_to(node_id, offset));
    }

    /// Reveal a vertical span inside one scroll container.
    pub fn scroll_into_view(
        &mut self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
    ) {
        self.command(Command::scroll_into_view(
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
        ));
    }

    /// Reveal a vertical span inside one scroll container and snap movement to a fixed row height.
    pub fn scroll_into_view_snapped(
        &mut self,
        node_id: NodeId,
        target_y: f32,
        target_height: f32,
        margin_top: f32,
        margin_bottom: f32,
        snap_y: f32,
    ) {
        self.command(Command::scroll_into_view_snapped(
            node_id,
            target_y,
            target_height,
            margin_top,
            margin_bottom,
            snap_y,
        ));
    }

    /// Reveal a fixed-stride row inside one scroll container with directional context rows.
    pub fn scroll_fixed_row_into_view(
        &mut self,
        node_id: NodeId,
        row_index: usize,
        row_stride: f32,
        leading_context_rows: usize,
        trailing_context_rows: usize,
        direction: i32,
    ) {
        self.command(Command::scroll_fixed_row_into_view(
            node_id,
            row_index,
            row_stride,
            leading_context_rows,
            trailing_context_rows,
            direction,
        ));
    }

    /// Request runtime exit.
    pub fn exit(&mut self) {
        self.command(Command::exit());
    }

    pub(super) fn into_command(self) -> Command<Message> {
        Command::batch(self.commands)
    }
}
