use super::super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    runtime::{
        ConfirmationButtons, ConfirmationLevel, ConfirmationResponse, FileDialogRequest,
        PlatformCompletion, PlatformRequest, PlatformResponse, PlatformResult,
        PlatformServiceFallback, TaskPriority,
    },
};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
    Message: Send + 'static,
{
    pub(super) fn request_app_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<Message>,
    ) -> Result<(), PlatformServiceFallback<Message>> {
        if !self.runtime.is_alive() || !self.runtime.can_spawn_business_tasks() {
            return Err(Box::new((request, on_completed)));
        }
        let runtime = std::sync::Arc::downgrade(&self.runtime);
        let _ = self.runtime.spawn_business_task(
            "radiant-platform-service",
            TaskPriority::Interactive,
            None,
            move || {
                let response = perform_platform_request(request);
                if let Some(runtime) = runtime.upgrade() {
                    let _ = runtime.enqueue(on_completed(response));
                }
            },
        );
        Ok(())
    }
}

fn perform_platform_request(request: PlatformRequest) -> PlatformResult {
    match request {
        PlatformRequest::PickFolder(request) => pick_folder(request),
        PlatformRequest::PickFile(request) => pick_file(request),
        PlatformRequest::SaveFile(request) => save_file(request),
        PlatformRequest::OpenPath(path) => open_path(path),
        PlatformRequest::RevealPath(path) => reveal_path(path),
        PlatformRequest::OpenUrl(url) => open_url(url),
        PlatformRequest::CopyText(text) => copy_text(text),
        PlatformRequest::Confirm(request) => confirm(request),
    }
}

fn pick_folder(request: FileDialogRequest) -> PlatformResult {
    let Some(path) = apply_file_dialog_request(rfd::FileDialog::new(), request).pick_folder()
    else {
        return Ok(PlatformResponse::Canceled);
    };
    Ok(PlatformResponse::Path(path))
}

fn pick_file(request: FileDialogRequest) -> PlatformResult {
    let Some(path) = apply_file_dialog_request(rfd::FileDialog::new(), request).pick_file() else {
        return Ok(PlatformResponse::Canceled);
    };
    Ok(PlatformResponse::Path(path))
}

fn save_file(request: FileDialogRequest) -> PlatformResult {
    let Some(path) = apply_file_dialog_request(rfd::FileDialog::new(), request).save_file() else {
        return Ok(PlatformResponse::Canceled);
    };
    Ok(PlatformResponse::Path(path))
}

fn apply_file_dialog_request(
    mut dialog: rfd::FileDialog,
    request: FileDialogRequest,
) -> rfd::FileDialog {
    if let Some(title) = request.title {
        dialog = dialog.set_title(title);
    }
    if let Some(directory) = request.directory {
        dialog = dialog.set_directory(directory);
    }
    if let Some(filename) = request.filename {
        dialog = dialog.set_file_name(filename);
    }
    for filter in request.filters {
        let extensions = filter
            .extensions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        dialog = dialog.add_filter(filter.name, extensions.as_slice());
    }
    dialog
}

fn open_path(path: std::path::PathBuf) -> PlatformResult {
    open::that(path).map_err(|err| err.to_string())?;
    Ok(PlatformResponse::Completed)
}

fn reveal_path(path: std::path::PathBuf) -> PlatformResult {
    if !path.exists() {
        return Err(format!("Path not found: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", windows_explorer_target(&path)))
            .status()
            .map_err(|err| format!("Failed to launch explorer: {err}"))?;
        if status.success() {
            Ok(PlatformResponse::Completed)
        } else {
            Err(format!(
                "Explorer exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|err| format!("Failed to launch Finder: {err}"))?;
        if status.success() {
            Ok(PlatformResponse::Completed)
        } else {
            Err(format!(
                "Finder exited unsuccessfully for {}",
                path.display()
            ))
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let target = if path.is_dir() {
            path.as_path()
        } else {
            path.parent()
                .ok_or_else(|| String::from("Unable to resolve parent directory"))?
        };
        open::that(target)
            .map_err(|err| format!("Could not reveal path {}: {err}", path.display()))?;
        Ok(PlatformResponse::Completed)
    }
}

fn open_url(url: String) -> PlatformResult {
    open::that(url).map_err(|err| err.to_string())?;
    Ok(PlatformResponse::Completed)
}

fn copy_text(text: String) -> PlatformResult {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("Failed to open clipboard: {err}"))?;
    clipboard
        .set_text(text)
        .map_err(|err| format!("Failed to copy text: {err}"))?;
    Ok(PlatformResponse::Completed)
}

fn confirm(request: crate::runtime::ConfirmDialogRequest) -> PlatformResult {
    let level = match request.level {
        ConfirmationLevel::Info => rfd::MessageLevel::Info,
        ConfirmationLevel::Warning => rfd::MessageLevel::Warning,
        ConfirmationLevel::Error => rfd::MessageLevel::Error,
    };
    let buttons = match request.buttons {
        ConfirmationButtons::Ok => rfd::MessageButtons::Ok,
        ConfirmationButtons::OkCancel => rfd::MessageButtons::OkCancel,
        ConfirmationButtons::YesNo => rfd::MessageButtons::YesNo,
    };
    let result = rfd::MessageDialog::new()
        .set_title(request.title)
        .set_description(request.message)
        .set_level(level)
        .set_buttons(buttons)
        .show();
    let response = match result {
        rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => {
            ConfirmationResponse::Accepted
        }
        rfd::MessageDialogResult::No => ConfirmationResponse::Rejected,
        rfd::MessageDialogResult::Cancel | rfd::MessageDialogResult::Custom(_) => {
            ConfirmationResponse::Canceled
        }
    };
    Ok(PlatformResponse::Confirmation(response))
}

#[cfg(target_os = "windows")]
fn windows_explorer_target(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}
