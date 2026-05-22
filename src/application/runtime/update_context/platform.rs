use crate::runtime::{
    Command, ConfirmDialogRequest, DragRequest, ExternalDragOutcome, ExternalDragRequest,
    FileDialogRequest, PlatformRequest, PlatformResponse,
};

use super::UpdateContext;

impl<Message> UpdateContext<Message> {
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

    /// Begin a runtime-owned pointer drag preview session.
    pub fn begin_drag(&mut self, request: DragRequest) {
        self.command(Command::begin_drag(request));
    }

    /// End any active runtime-owned pointer drag preview session.
    pub fn end_drag(&mut self) {
        self.command(Command::end_drag());
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

    /// Ask the platform integration to choose an existing file.
    pub fn pick_file(
        &mut self,
        request: FileDialogRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::PickFile(request), on_completed);
    }

    /// Ask the platform integration to choose a save path.
    pub fn save_file(
        &mut self,
        request: FileDialogRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::SaveFile(request), on_completed);
    }

    /// Ask the platform integration to open a local path with the OS shell.
    pub fn open_path(
        &mut self,
        path: impl Into<std::path::PathBuf>,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::OpenPath(path.into()), on_completed);
    }

    /// Ask the platform integration to open a URL with the OS shell.
    pub fn open_url(
        &mut self,
        url: impl Into<String>,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::OpenUrl(url.into()), on_completed);
    }

    /// Ask the platform integration to show a confirmation dialog.
    pub fn confirm(
        &mut self,
        request: ConfirmDialogRequest,
        on_completed: impl FnOnce(Result<PlatformResponse, String>) -> Message + Send + 'static,
    ) {
        self.platform_request(PlatformRequest::Confirm(request), on_completed);
    }
}
